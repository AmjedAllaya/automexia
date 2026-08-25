# Automation Studio and domain-extension architecture

- Status: Proposed post-first-stable-release architecture; AS0 feasibility may
  proceed earlier, but no editor, language server, public add-on, or script-
  execution surface is authorized or shipped
- Research snapshot: 2026-08-24
- Durable decision: [Proposed ADR 0030](adr/0030-automation-studio-domain-extension-boundary.md)
- Verification owner: [Automation Studio testing](AUTOMATION-STUDIO-TESTING.md)

## Outcome

Automexia should make scripts and configuration easier to create, understand,
review, and run without turning the terminal into a large, always-on IDE. The
recommended product is an optional **Automation Studio** embedded in the
Automexia window. It can sit beside terminal panes, open as a workspace view,
and disappear completely when it is disabled.

Automation Studio is a general editing foundation. The DevOps/SRE extension is
a separate domain extension that contributes infrastructure knowledge,
reviewable operations, and environment context. A DevOps/SRE Pack may install
both for a simple first-run experience, but the packages remain independently
owned, enabled, updated, disabled, and uninstalled.

## Delivery position

AS0 research, dependency comparison, native-host feasibility work, and threat
modeling may continue while Automexia closes its first stable v0.4 release.
Those activities must remain disposable and must not add a production
dependency, runtime path, release claim, or v0.4 blocker.

AS1 and later product implementation follows that stable terminal release and
reuses the independently proven extension, workspace, process, and DevOps
boundaries. The first releasable Studio product slice is AS1 plus AS2: trusted
document ownership and a minimal editor with save, recovery, and terminal-only
fallback. That slice should ship before a dedicated video-editing extension.

Video research and terminal-native media workflows may continue in parallel.
AS3-AS6 are not blanket prerequisites for video: language intelligence,
DevOps/SRE execution, domain packs, debugging, remote access, collaboration,
and AI remain independently gated. Studio and future video work may share only
generic core-owned workspace, file, task, progress, cancellation, recovery,
and extension-lifecycle services. Video must not depend on Studio's editor
webview, document-view state, or language-server internals.

This page describes a target architecture. It does not change the current
capability boundary or authorize implementation. In particular, any replacement
or extension of [ADR 0003](adr/0003-extension-capability-and-threading.md), plus
[ADR 0023](adr/0023-typed-automation-and-declarative-workspaces.md),
[ADR 0029](adr/0029-sandboxed-signed-ecosystem-boundary.md), and
[ADR 0030](adr/0030-automation-studio-domain-extension-boundary.md) must pass
their stated acceptance gates before the corresponding authority exists.

## Current repository truth

| Area | Current status | Evidence and consequence |
|---|---|---|
| Terminal, panes, tabs, PTY, renderer, configuration | **Available now / implemented at the current product boundary** | Preserve the current owners and hot-path rules in [Architecture](ARCHITECTURE.md). Studio failure must not affect terminal input/output. |
| Private first-party extension API and bounded runtime | **Fully done at the D1 source boundary; not a public SDK** | Existing typed, renderer-neutral contracts may be extended only after the new capability decision is accepted. They do not currently grant file write, arbitrary process, webview, or third-party authority. |
| First-party SSH, cloud, Kubernetes/OpenShift, Teleport, and provider-aware action foundations | **Implemented or source-complete at their documented nonactivated boundaries; release evidence remains partial** | Reuse their public context, exact-target, risk, review, and lifecycle contracts. Do not duplicate provider clients or credential stores inside Studio. |
| M6 typed automation and declarative workspaces | **Partially done overall; review/edit CLI and Hub surface complete locally, execution disabled** | Extend its immutable revisions, fingerprints, review generations, preview-first CAS manager, and no-hook rule instead of adding a second automation store. |
| CP5 native-editor suggestion bridge | **Proposal-only; no runtime** | In CP5, “editor” means the shell-owned command line. It is not a file editor and does not authorize Studio. |
| D7/CP6 public extension ecosystem | **Partially done only at a proposal/policy boundary; no runtime** | Third-party Studio add-ons require ADR 0029. First-party Studio work does not turn the private Rust API into a public SDK. |
| Automation Studio | **AS0 partially done only as architecture, research, and test planning; AS1-AS6 not done** | No CodeMirror, Monaco, Wry, LSP, DAP, editor host, document service, Studio UI, or run path has been added to the workspace. |

Roadmap prose is not implementation evidence. The
[feature catalog](FEATURES.md), [roadmap](ROADMAP.md), and
[phase audit](PHASE-IMPLEMENTATION-AUDIT.md) keep these statuses synchronized.

## Product composition

```text
Automexia desktop composition root
|
+-- terminal core and existing workspace surfaces
|
+-- proposed core brokers
|   +-- DocumentService and recovery
|   +-- WorkspaceTrust and capability policy
|   +-- EditorSurfaceHost
|   +-- LanguageToolBroker
|   +-- existing ExternalToolRunner / session broker
|   +-- credential-reference and audit/receipt services
|
+-- Automation Studio (optional first-party foundation extension)
|   +-- file editor, tabs, split views, diff and diagnostics presentation
|   +-- no direct filesystem, process, network, credential, provider, or PTY authority
|
+-- DevOps/SRE (optional first-party domain extension)
|   +-- infrastructure context, templates, policies, plans and review models
|   +-- works in terminal-only mode when Studio is absent
|
+-- language and tool add-ons
|   +-- grammars, language-server descriptors, formatters and validators
|   +-- first-party initially; third-party delivery waits for D7
|
+-- DevOps/SRE Pack (metadata-only convenience package)
    +-- installs a compatible set; grants no additional authority
```

### Why Studio is separate from DevOps/SRE

Putting the editor directly inside the DevOps/SRE extension would make a domain
package own a generic document surface, complicate reuse by data, media,
research, and other future domains, and make terminal-only DevOps unnecessarily
heavy. Putting it in the terminal core would increase startup, binary, browser,
and security costs for every user.

The separate-foundation model gives each package one reason to change:

- the core owns authority, persistence, isolation, and platform integration;
- Automation Studio owns the editing experience and editor-neutral projections;
- DevOps/SRE owns domain meaning and target-aware workflows;
- a language/tool add-on owns one bounded integration contract;
- the Pack owns only compatible dependency selection and first-run defaults.

The DevOps/SRE extension must still provide useful terminal-native actions,
templates, diagnostics, and review flows when Studio is unavailable. Studio must
still edit ordinary files when DevOps/SRE is unavailable.

## User experience boundary

Studio is integrated into the Automexia application, not drawn into the PTY
cell grid and not opened in a separate IDE window by default. A workspace may
show terminal panes, a Studio editor, and a review/result surface together. The
user can close Studio and return to the unchanged terminal layout.

The first Automation Studio product release should support:

1. opening an explicitly selected local folder or file;
2. text editing, search, replace, tabs, split views, save, diff, and recovery;
3. syntax highlighting and local diagnostics from explicitly enabled add-ons;
4. visible workspace trust and tool state;
5. reviewable check, format, lint, test, plan, and run intents after later gates;
6. keyboard-only use, screen-reader semantics, high contrast, scaling, reduced
   motion, IME, Unicode, bidi-safe display, and reliable focus restoration.

An optional “open in external editor” action may be added later, but it is an
explicit user choice and not a substitute for the embedded product contract.
Neovim, Vim, Helix, or another terminal editor remain valid terminal-native
workflows and must continue to work without Studio.

## Proposed core owners

| Owner | Owns | Must not own |
|---|---|---|
| `DocumentService` | Canonical document URI, encoding/newline metadata, text revision, dirty state, external-change detection, atomic save, recovery journal, save conflict, and view subscriptions | Editor DOM state, provider meaning, process launch, secrets, or terminal history |
| `WorkspaceTrust` | Stable workspace identity, restricted/trusted state, trust receipt, revocation, path/source changes, and policy generation | A blanket approval for tools, network, providers, credentials, or production |
| `EditorSurfaceHost` | Native child-surface lifecycle, bounds/scale/focus, crash/restart, typed IPC transport, locally bundled assets, mediated user-gesture clipboard, and accessibility bridge | Filesystem or unrestricted clipboard APIs exposed to JavaScript, arbitrary navigation, generic host-object proxies, provider calls, or PTY ownership |
| `LanguageToolBroker` | Language-server/tool descriptors, exact executable selection, JSON-RPC framing, limits, cancellation, version state, workspace/document routing, and mediation of server requests | Direct trust decisions, arbitrary server commands, unreviewed workspace edits, ambient environment, or credentials |
| `ExternalToolRunner` | Existing exact-executable/exact-argv process boundary, protected stdin, environment allowlist, deadlines, output ceilings, descendant cleanup, and redacted outcomes | Shell command strings, implicit Enter, extension-controlled spawn, or hidden elevation |
| terminal/session broker | Existing PTY/session ownership and explicit reviewed placement of a new terminal session | Treating editor text as terminal input or allowing Studio to type into an existing PTY |
| credential-reference service | Opaque, scoped references to platform/external credential owners | Broker-resolved secret values injected into documents, manifests, editor IPC, logs, recovery, or extension state |
| audit/receipt service | Redacted immutable operation identity, inputs, decision generation, result, and cleanup state | Raw file contents, terminal history, environment values, tokens, or provider output |

The application remains the composition root. No editor, language, DevOps, or
third-party package receives window, renderer, PTY, process, credential-store,
or unrestricted filesystem handles.

## Document model and recovery

`DocumentService` is the only canonical owner for an open document. Editor
views are disposable projections. A URI has one current revision even when it
is visible in several Studio views or windows.

The target flow is:

```text
explicit file/folder grant
  -> bounded read and encoding/newline validation
  -> DocumentService revision N
  -> one or more editor view mirrors
  -> validated ordered deltas
  -> DocumentService revision N+1
  -> atomic save or explicit conflict/recovery state
```

Required invariants:

- every delta binds document ID, base revision, view ID, workspace, window,
  message sequence, and generation;
- stale, duplicated, oversized, out-of-order, or cross-document deltas fail
  without changing the canonical revision;
- the editor view never writes files directly and never treats browser storage
  as authoritative;
- saves use same-directory temporary output, flush/close as required, atomic
  replacement where the platform supports it, permission preservation, and a
  visible fallback when atomic replacement is unavailable;
- an external modification creates an explicit compare/reload/overwrite choice;
  it is never silently overwritten;
- recovery belongs to the core, is bounded and user-private, never mixes another
  document or broker-resolved credential value, supports explicit disable and
  expiry, and is deleted after a confirmed save/close or exact expiry;
- symlink, path traversal, mount/share, permission, read-only, disk-full,
  encoding, very-large-file, binary-file, and deleted/renamed-file states are
  explicit;
- remote documents require a future content-provider contract with revisioned
  read/write/compare semantics; a local path is never fabricated for them.

The initial execution path accepts only a saved canonical revision. Running
unsaved editor text is deferred until an immutable temporary-artifact lifecycle
and its cleanup/recovery evidence are independently accepted.

## Editor surface and technology decision

### Editor library

[CodeMirror 6](https://codemirror.net/) is the recommended primary editor
foundation for the first proof and product slice. It is MIT licensed, modular,
supports mobile browsers, and allows Automexia to include only the state, view,
commands, language, lint, and accessibility pieces the product needs. Its
[system guide](https://codemirror.net/docs/guide/) makes the state/view split
explicit, which fits the core-owned document model.

[Monaco Editor](https://github.com/microsoft/monaco-editor) remains a serious
desktop-only comparison candidate. It provides a richer VS Code-derived surface
and native LSP APIs, but its official project does not support mobile browsers,
and its wider worker/service surface increases integration and resource work.
It should replace CodeMirror only if the AS0 benchmark and native accessibility
matrix show a material workflow benefit that justifies the cost.

Automexia should not build a text editor widget, selection engine, IME model,
syntax parser ecosystem, or accessibility implementation from scratch. A native
text widget may still be used for small forms, not as the Studio document
editor.

### Native host

[Wry](https://github.com/tauri-apps/wry) is the first host candidate because it
wraps system webviews and accepts raw window handles used by Rust windowing
stacks. It is not approved by this document. Wry requires an event loop and a
window handle, or a GTK container for Linux X11 and Wayland. Automexia's current
custom window/renderer composition therefore needs a real AS0 proof on:

- Windows with the serviced WebView2 runtime;
- macOS with WKWebView;
- Linux X11 with WebKitGTK;
- Linux Wayland with the required GTK container and real scale/resize/focus
  behavior.

Raw child-window success on Windows, macOS, or X11 is not Wayland proof. If Wry
cannot satisfy native focus, IME, scaling, accessibility, resize, z-order, GPU,
cleanup, and package requirements, keep `EditorSurfaceHost` stable and evaluate
a platform-specific Linux adapter. Do not force a fragile universal host into
the terminal core.

The hosted content is locally bundled and uses an app-owned origin with a strict
Content Security Policy and no remote scripts, fonts, frames, workers, or
navigation unless an exact later requirement is reviewed. Host messages are
versioned typed values with origin, request, window, workspace, document,
revision, generation, and byte validation. There is no generic JavaScript-to-
native proxy. This follows Microsoft's guidance to
[treat WebView2 content as untrusted, validate origins/messages, and avoid
generic proxies](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security).

At most one Studio webview host should exist per top-level window in the first
release. Multiple files and split editor views live inside that host. A host
crash loses only disposable view state; canonical documents and recovery remain
in the core and can recreate the view.

## Language intelligence and tool add-ons

Automexia should adopt the
[Language Server Protocol 3.18](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/)
at a core broker rather than invent per-language completion, diagnostics,
navigation, rename, and code-action protocols. The editor translates UI events
to Automexia document operations; the broker translates approved language
features to bounded LSP JSON-RPC.

A language/tool add-on declares data, not spawn authority:

- stable add-on and publisher identity;
- compatible Studio, broker, protocol, and platform versions;
- language IDs, file patterns, grammars, comments, brackets, indentation, and
  optional snippets/templates;
- an exact language-server descriptor, supported versions, startup arguments,
  workspace requirements, and declared capability needs;
- formatter, linter, validator, or CLI descriptors with exact operation kinds;
- output schemas, diagnostic mapping, limits, offline behavior, health checks,
  disable/uninstall behavior, and license/provenance metadata.

The broker resolves an installed executable through the existing tool policy,
starts it only for an explicitly opened trusted workspace and enabled language,
uses an allowlisted environment, protects stdin/stdout framing, bounds messages,
queues, memory, processes, retries, and logs, and cancels obsolete document or
workspace generations. A language server cannot directly:

- apply a workspace edit;
- run `workspace/executeCommand` or another tool;
- change configuration or open an external URI;
- request credentials, providers, terminals, network, or elevation;
- publish diagnostics for another document generation.

The descriptor declares whether the server needs only synchronized open
documents, direct workspace reads, network, or child processes. Exact working
directory and environment are scoping inputs, not a filesystem sandbox. A direct
workspace scan therefore needs separate visible review, and any claim that a
server cannot read outside the workspace requires native OS/container sandbox
evidence. Without that confinement, the UI must say that the installed server
runs with the user's operating-system authority; restricted mode keeps it off.

Those requests re-enter a typed core decision path and may be denied. Starter
first-party add-ons should focus on shell scripts, Python, YAML, JSON, TOML,
Dockerfiles, and HCL/OpenTofu/Terraform workflows after the broker passes its
gates. Add languages incrementally instead of shipping a large server bundle.

The [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/)
is a later AS6 option. Debug adapters add process attachment, child execution,
ports, source mapping, expression evaluation, and secret exposure risks; they do
not inherit LSP approval.

## DevOps/SRE integration

The DevOps/SRE extension contributes domain meaning through versioned, bounded
records. With Studio absent, it continues to offer terminal-native Quick
Actions, templates, provider context, review, and explicit insertion. With
Studio present, a capability handshake may additionally contribute:

- script and configuration templates with provenance and required variables;
- tool/language recommendations, never silent installation;
- diagnostics and links to public documentation;
- environment-aware review decorations that show account, project, cluster,
  namespace, host, workspace, and production risk without exposing secrets;
- typed checks, lints, diffs, plans, tests, and run intents;
- provider-specific result parsers that convert bounded external output into
  untrusted typed summaries;
- exact cleanup, retry, and recovery guidance.

It does not own documents, the editor host, language-server processes, terminal
sessions, provider credentials, or process spawning. A DevOps decoration cannot
make an operation safe, mint a capability, or bypass final revalidation.

Infrastructure actions should prefer reviewable semantics. OpenTofu/Terraform
and Kubernetes changes should expose plan/diff before apply when the underlying
tool provides a reliable contract. Ansible, shell, and arbitrary scripts that
cannot promise an accurate plan must say so plainly; a syntax check or dry run
must never be labeled a complete production preview.

## Typed execution contract

Studio never builds or evaluates a shell command string. A future run begins as
a `ScriptRunIntent` containing at least:

| Field group | Required binding |
|---|---|
| Identity | intent ID, extension/add-on identity, workspace ID, document URI, saved revision and content digest |
| Tool | logical tool/interpreter ID, resolved executable identity and version, exact argument array, protected stdin mode |
| Location | validated working-directory reference and explicitly selected target context |
| Environment | Environment Capsule ID/revision and a named allowlisted environment policy, never a raw inherited map |
| Secrets | opaque credential-reference IDs with purpose, target, lifetime, and redaction policy; never values |
| Operation | `check`, `format`, `lint`, `test`, `plan`, `apply`, or `run`, including declared side-effect/risk class |
| Limits | startup/idle/total deadlines, input/output/log ceilings, descendant/process/concurrency limits, cancellation owner |
| Review | immutable review generation, workspace-trust generation, capability grant, production flag, preview/plan digest, and expiry |
| Result | redacted receipt ID, exit/ambiguous state, bounded typed output, affected target, cleanup state, and retry eligibility |

Final execution revalidates every material field, current file digest, workspace
trust, executable identity/version, capsule/target generation, capability,
preview/plan digest, expiry, and production confirmation. Any change invalidates
review. The runner starts a new supervised process or a new explicitly placed
terminal session; it never types into an existing PTY and never presses Enter.

`format` is a document edit, not merely a process result. The formatter output
must be parsed as a bounded proposed edit/diff for the exact input revision and
applied through `DocumentService` only after policy and stale-revision checks.
`apply` is separately authorized from `plan`.

## Four execution tiers

| Tier | Purpose | Authority and placement |
|---|---|---|
| 1. Declarative contribution | Grammars, snippets, templates, menus, schemas, tool descriptors, settings, and help | Parsed as strict bounded data; no code execution |
| 2. Sandboxed component | Deterministic bounded transformation or validation that cannot be expressed as data | Future D7 custom-WIT component with no default WASI; fuel, deadline, memory/output, queue, cancellation, and host-transfer limits |
| 3. Supervised external tool | LSP servers, formatters, linters, validators, CLIs, tests, plans, and scripts | Existing core runner/broker with exact executable and argv, explicit workspace/target/capability, allowlisted environment, protected stdin, output limits, and descendant cleanup |
| 4. Native first-party core | PTY, system keychain adapter, native file dialog, window/webview host, accessibility adapter, and platform process cleanup | Small reviewed platform adapters; never the public extension format |

Signing does not move a package to a more privileged tier. Third-party native
libraries and arbitrary install scripts are not valid public extension payloads.

Supervision is not sandboxing. A tier-3 process normally retains the authority
of the operating-system account unless a separately evidenced sandbox/container
profile confines it. Automexia can restrict what it supplies, observes, and
authorizes, but must not claim that exact argv, cwd, environment, deadlines, or
process-tree cleanup alone prevents the tool from accessing other user resources.

## Workspace trust and capabilities

Opening a folder allows bounded viewing and editing in restricted mode; it does
not authorize code execution. This follows the useful separation in the
[VS Code Workspace Trust model](https://code.visualstudio.com/api/extension-guides/workspace-trust)
without copying VS Code's extension API.

Restricted mode denies language servers, workspace-defined executable paths,
formatters, linters, tasks, scripts, hooks, providers, network, secret
resolution, debug adapters, and production operations. Static highlighting,
plain editing, diff, search, and safe metadata may remain available.

Trust is necessary but not sufficient. A grant also binds publisher, extension,
version, package digest where applicable, capability, workspace, exact path or
target scope, profile, operation kind, quotas, review generation, and expiry.
Capability growth, package digest change, workspace relocation/symlink change,
tool identity change, target change, or trust revocation requires fresh review.

No package may receive a broad “DevOps” or “Studio” permission. Capabilities are
specific, such as read an explicitly granted workspace, propose a document
edit, request one named tool operation, read one public provider context, or
open one new supervised terminal session.

## Lifecycle, failure, and fallback

Activation is lazy: installing Studio does not create a webview, scan a
workspace, start a language server, or launch a tool. Opening an editor surface
loads the local UI; opening a supported document may start only explicitly
enabled language features after trust checks.

Every component has install, verify, enable, activate, suspend, update, disable,
uninstall, rollback, crash, and shutdown ownership. Required fallback behavior:

- terminal input/output, panes, native shells, CP1 completion, and current
  first-party extensions remain usable when Studio is absent or failed;
- disabling DevOps/SRE removes its contributions and authority without closing
  ordinary documents;
- disabling a language add-on stops and joins its servers, removes diagnostics
  and grants, and leaves plain editing available;
- a Studio webview crash recreates disposable views from core state and does not
  lose confirmed saves or bounded recovery state;
- uninstall removes only exact Automexia-owned package/cache/grant generations,
  never user files, shell profiles, external tools, provider state, or
  credentials;
- updates do not silently change capabilities, tool descriptors, data flow,
  quotas, signer, risk, or default activation.

## Resource and security contract

Before AS1 source work, a versioned machine fixture must lock numeric ceilings
for document bytes, open documents/views, delta and IPC bytes, outstanding
requests, recovery bytes/age, diagnostics, edits, LSP messages, server count,
processes, output/logs, concurrency, timeouts, retries, crashes, webviews,
storage, and shutdown. There are no unbounded defaults.

The threat model must cover at least:

| Threat | Required control |
|---|---|
| Malicious workspace or configuration | Restricted mode, ignored execution-bearing workspace settings, exact trust receipt, no automatic tasks/hooks/tools |
| Path traversal, symlink/TOCTOU, hostile shares | Explicit grants, canonical/no-follow containment where applicable, source identity, final revalidation, atomic save/conflict state |
| Webview compromise or hostile message | Local assets, strict origin/CSP/navigation, typed versioned IPC, message ceilings, no generic proxy, no direct file/network/process/unrestricted-clipboard API |
| Language-server compromise | Declared file/network/child authority, restricted-mode denial, supervised process, bounded JSON-RPC, mediated requests/edits, native sandbox evidence for any confinement claim, cancellation, descendant cleanup, no credentials/ambient environment |
| Script/argument injection | Typed executable plus exact argv, no shell evaluation, validated cwd, allowlisted environment, protected stdin, no implicit Enter |
| Stale document, plan, target, or capability | Revision/digest/generation binding and final revalidation before every edit or execution |
| Secret disclosure | Opaque references, purpose-bound resolution at the runner, canary tests, user-private document/recovery handling, structured redaction, and no broker-resolved secret in document/editor IPC/log/recovery |
| Production-target confusion | Visible target and risk, separate apply grant, typed production confirmation, plan digest, expiry, redacted receipt |
| Resource amplification | Numeric limits, bounded queues/concurrency/caches, cancellation, crash quarantine, joined teardown, long-session resource evidence |
| Package/update compromise | ADR 0029 signed immutable bundles, provenance/SBOM, current revocation, capability diff, rollback floor, exact uninstall |

## Platform and lightweight profiles

The ordinary terminal remains the minimum profile. Studio and its dependencies
must be optional at build/package and runtime boundaries where practical.

| Profile | Intended behavior | Current claim |
|---|---|---|
| Terminal-only | Existing terminal, native editors, current extensions and actions; no webview or language servers | Current product direction; must remain the fallback |
| Studio Lite | Embedded editing, search, diff, recovery, static grammar, no background language server or execution | Planned, not implemented |
| Studio Full | Explicit language servers and reviewed tools/DevOps workflows | Planned, gated by AS3-AS5 |
| Remote/mobile client | Touch-friendly editor UI backed by an authenticated remote Automexia host | Research only; requires its own protocol, privacy, offline, conflict, and device-security ADR |

Raspberry Pi or another supported ARM Linux computer may become a native host
only after real ARM64 build, package, webview/GPU, PTY, IME/accessibility,
performance, thermal, memory, and cleanup evidence. A Nintendo Switch is not a
supported target and would additionally need a supported operating system,
drivers, packaging, and input model. An Arduino-class microcontroller cannot
host the desktop terminal/editor architecture; it may be a device managed by a
future serial extension. Smartphones fit a future remote/client profile better
than an immediate port of the full desktop application. These are directions,
not compatibility claims.

## Implementation sequence

AS0 may proceed before the first stable terminal release because it produces no
product runtime. AS1-AS2 are post-release product work and must not delay v0.4.
The AS2 exit gate is the minimum Studio milestone that precedes a released
video-editing extension; AS3-AS6 may continue independently afterward.

| Phase | Deliverable | Exit before the next phase |
|---|---|---|
| AS0 — decision and feasibility | This architecture, ADR 0030, threat/evidence plan, CodeMirror-vs-Monaco measurement, Wry/native-host proof on Windows/macOS/Linux X11/Linux Wayland, dependency/license/MSRV/size/startup review, and numeric machine contract | ADR acceptance, reviewed native proof, no hot-path regression, selected host/editor rationale, rollback plan |
| AS1 — neutral contracts | `DocumentService`, workspace trust, grants, recovery, typed IPC, lifecycle, surface abstraction, exact limits and hostile fixtures; no product editor | Pure/model/property/fuzz/mutation evidence; atomic save/recovery/crash cleanup; no process/network authority |
| AS2 — minimal Studio | Locally bundled editor with open/edit/search/diff/save/recovery and accessibility semantics; no LSP or execution | Four-platform native UI/IME/scale/focus/accessibility/resource/package evidence; terminal-only fallback |
| AS3 — language intelligence | `LanguageToolBroker`, starter first-party language packs, bounded LSP sync/diagnostics/completion/navigation/rename/code action mediation | Fake/malicious server, cancellation/stale, multi-workspace isolation, native server lifecycle, performance and uninstall evidence |
| AS4 — DevOps/SRE scripting | Separate domain extension, `ScriptRunIntent`, check/format/lint/test/plan/run review, exact runner integration and receipts | Exact-argv/security/resource/native/provider fixtures; no existing-PTY input; saved-revision and production review proof |
| AS5 — domain packs | Terraform/OpenTofu, Kubernetes/OpenShift, Ansible, cloud, policy and other independently gated integrations; metadata-only Pack | Per-tool version/offline/plan/apply/rollback evidence, provider isolation, real controlled targets, disable/uninstall |
| AS6 — advanced and remote | DAP/debugging, remote content providers, mobile client, collaboration, optional AI assistance | Separate ADRs and threat models for attachment/evaluation, remote transport, conflicts, privacy, multi-user authority, and AI; none inherit AS0-AS5 approval |

First-party AS1-AS4 do not need to wait for a public marketplace, but they do
need ADR 0030 and every existing protected capability gate. Third-party Studio,
language, or DevOps add-ons additionally wait for ADR 0029 and D7 package/runtime
evidence.

## Alternatives considered

- **Put Studio inside the DevOps/SRE extension:** rejected as the default because
  a general document editor would become coupled to one domain and terminal-only
  DevOps would inherit its weight and platform risk.
- **Put a full IDE in the terminal core:** rejected because every user would pay
  startup, size, browser, attack-surface, and maintenance costs, and the core
  would become a second provider/tool owner.
- **Open a separate IDE application:** retained only as an explicit external-
  editor action. It loses the integrated target/review/workspace experience the
  feature is meant to provide.
- **Render the file editor inside the PTY grid:** rejected because terminal cells
  are not a document, IME, accessibility, multi-view, or trusted edit protocol.
- **Use Monaco first:** deferred behind AS0 comparison. It is strong for a
  desktop IDE surface but officially lacks mobile support and brings a wider
  service/worker integration than the initial Studio contract requires.
- **Embed Neovim as the product document owner:** rejected. Neovim remains an
  excellent terminal-native option and supports RPC plugin hosts, but adopting
  its buffer/session model would create a second process/editor authority and a
  different accessibility/UI contract. It may be offered as an external editor.
- **Build a native editor from scratch:** rejected because text layout,
  selection, IME, undo, syntax, accessibility, and platform behavior are mature
  specialist problems unrelated to Automexia's differentiating value.
- **Embed or fork a complete IDE workbench:** rejected for the first slice because
  it imports another window, process, extension, update, and authority model
  instead of fitting the existing Automexia composition root.
- **Give extensions direct files, processes, network, or webview APIs:** rejected.
  Extensions propose typed work; core brokers authorize and perform it.

## Primary references

- [CodeMirror](https://codemirror.net/) and its
  [system guide](https://codemirror.net/docs/guide/)
- [Monaco Editor](https://github.com/microsoft/monaco-editor)
- [Wry](https://github.com/tauri-apps/wry)
- [Language Server Protocol 3.18](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/)
- [Debug Adapter Protocol](https://microsoft.github.io/debug-adapter-protocol/)
- [VS Code Workspace Trust](https://code.visualstudio.com/api/extension-guides/workspace-trust)
- [VS Code Virtual Workspaces](https://code.visualstudio.com/api/extension-guides/virtual-workspaces)
- [Secure WebView2 applications](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security)
- [Wasmtime interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html)
  and [resource limiting](https://docs.wasmtime.dev/api/wasmtime/trait.ResourceLimiter.html)
- [WCAG 2.2](https://www.w3.org/TR/WCAG22/) and the
  [ARIA Authoring Practices tabs pattern](https://www.w3.org/WAI/ARIA/apg/patterns/tabs/)

These references support technology evaluation; they do not approve a package,
version, feature set, transitive dependency, or release claim. AS0 must record
the exact versions, licenses, advisories, provenance, features, MSRV, binary and
startup cost, update owner, native platforms, rollback, and replacement path
before any dependency enters the workspace.
