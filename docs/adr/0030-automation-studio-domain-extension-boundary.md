# ADR 0030: Automation Studio and domain-extension boundary

- Status: Proposed; source implementation and activation remain forbidden until
  explicit acceptance and AS0 native feasibility evidence
- Date: 2026-08-24

## Context

Automexia aims to make command-driven workflows faster and easier across
operations, automation, data, media, and other domains. DevOps/SRE users would
benefit from an integrated place to create and review shell scripts, Python,
YAML, infrastructure configuration, and automation files while retaining the
terminal, target context, and review state beside the document.

The repository does not currently contain a file-editor surface, document
service, webview host, language-server broker, Studio extension, or Studio run
path. The CP5 “native-editor suggestion bridge” in ADR 0025 refers only to the
shell-owned command line and is not a file editor. M6 contains review-only typed
automation contracts with product execution disabled. D7/CP6 contains a
proposal-only public package and sandbox contract; the current private Rust
extension API is not a public SDK.

Putting a general editor inside the DevOps/SRE extension would couple a reusable
workspace foundation to one domain. Putting it in the terminal core would make
every user pay its startup, binary, browser, resource, and security costs.
Allowing editor or domain extensions to access files, processes, credentials,
providers, terminal sessions, or webview host objects directly would conflict
with ADR 0003, the one ExternalToolRunner boundary, and the existing single-owner
architecture.

The complete proposed architecture and phase contract are in
[Automation Studio architecture](../AUTOMATION-STUDIO-ARCHITECTURE.md). The
future evidence ladder is in
[Automation Studio testing](../AUTOMATION-STUDIO-TESTING.md).

The product sequence also matters. Studio must not expand or delay the first
stable terminal release, while a future video-editing extension should not be
the first consumer that forces generic workspace, file, task, cancellation, or
recovery services into core.

## Proposed decision

If this ADR is explicitly accepted with the exact AS0 editor/host/dependency and
machine-contract evidence, Automexia will implement Automation Studio as an
optional first-party foundation extension embedded inside the Automexia window.
It is a document workspace beside terminal panes, not a renderer inside the PTY
grid and not a separate IDE application by default.

AS0 feasibility work may occur before Automexia's first stable v0.4 release
only as disposable, non-production evidence. AS1 and later product integration
follows that release. The first releasable Studio slice is AS1 plus AS2 and must
precede a dedicated video-editing extension. Video research may continue in
parallel, and AS3-AS6 do not become blanket prerequisites for video.

Studio and future video work may reuse generic core-owned workspace, file,
task, progress, cancellation, recovery, and extension-lifecycle services.
Video does not inherit or depend on Studio's editor webview, view state,
language-server broker, or DevOps/SRE package.

The DevOps/SRE extension remains a separate optional domain extension. It works
without Studio through terminal-native actions and review. When both are enabled,
a versioned capability handshake allows DevOps/SRE to contribute bounded
templates, diagnostics, target/risk context, typed plans and run intents. A
metadata-only DevOps/SRE Pack may install compatible versions of Studio,
DevOps/SRE, and selected language/tool add-ons, but the Pack grants no authority
and does not merge their ownership or lifecycle.

The application composition root owns all authority-bearing brokers:

- one canonical `DocumentService` per document URI for revisions, dirty state,
  atomic save, external conflict, bounded recovery, and view subscriptions;
- `WorkspaceTrust` for restricted/trusted state and revocable trust receipts;
- `EditorSurfaceHost` for native child-surface lifecycle, focus, scale,
  accessibility bridge, local assets, mediated user-gesture clipboard, typed
  IPC, crash recovery, and teardown;
- `LanguageToolBroker` for bounded LSP framing, exact tool descriptors,
  cancellation, stale-version rejection, and mediation of server requests;
- the existing `ExternalToolRunner`/session broker for typed executable identity,
  exact argument arrays, allowlisted environment, protected stdin, deadlines,
  output limits, descendant cleanup, and explicit new-session placement;
- opaque credential references and redacted audit/receipt ownership.

Automation Studio owns editor presentation, view-local selection/scroll/layout,
tabs, split views, diff, diagnostics presentation, and user gestures. It never
receives unrestricted filesystem, process, network, credential, provider,
renderer, window, accessibility-platform, or PTY handles. Canonical document
text lives in the core; the editor is a disposable revisioned mirror and browser
storage is never authoritative.

DevOps/SRE owns provider/tool meaning, templates, public target context, risk
classification, review models, and typed result interpretation. It does not own
documents, editor/webview lifecycle, language-server/tool processes, terminal
sessions, credentials, or provider login/networking.

Language/tool add-ons begin as strict declarative descriptors: identity,
compatibility, language/file metadata, grammars, server/formatter/linter/tool
descriptors, supported versions, exact operation kinds, schemas, limits,
offline/health behavior, provenance, and lifecycle. First-party packages may be
linked through private contracts after this ADR's acceptance. Third-party
packages require separate acceptance and implementation of ADR 0029; this ADR
does not expose the private Rust ABI as a public SDK.

CodeMirror 6 is the proposed first editor candidate because its modular MIT-
licensed state/view design and mobile support fit a lightweight optional
surface. Monaco remains the measured desktop comparison. Wry is the proposed
system-webview host candidate, subject to real Windows, macOS, Linux X11, and
Linux Wayland/GTK-container proof on Automexia's current window stack. These are
conditional build/wrap/adopt choices, not dependency approvals. The
`EditorSurfaceHost` boundary must allow a platform-specific replacement if the
candidate fails focus, IME, scale, accessibility, GPU, package, or cleanup gates.

Hosted content is locally bundled under an app-owned origin with strict content
security policy and navigation. IPC is typed, versioned, origin-checked, bounded,
sequenced, cancellable, and bound to window, workspace, document, revision, and
generation. There is no generic JavaScript/native proxy and no extension-
supplied arbitrary HTML or JavaScript.

Language features use an Automexia-owned broker over LSP 3.18 rather than
editor-specific provider authority. The broker starts explicitly enabled,
supported servers only for trusted workspaces, through the core process owner.
Server requests to edit workspace files, execute commands, change configuration,
open URIs, or widen access re-enter typed host policy and default to denial.

A language-server descriptor declares synchronized-document, direct-workspace-
read, network, and child-process needs. Exact cwd/environment and supervision do
not confine filesystem or network access. Any containment claim requires native
OS/container sandbox evidence; otherwise the review must state that the installed
server runs with the user's operating-system authority, and restricted mode
keeps it disabled.

Debug Adapter Protocol support is deferred to a separate AS6 decision because
attach, expression evaluation, debug consoles, ports, and child execution add
new authority.

Execution has four tiers:

1. declarative data with no code authority;
2. future bounded custom-WIT Wasm components after ADR 0029, with no default
   WASI and explicit fuel/deadline/memory/output/queue/cancellation limits;
3. supervised external language servers, formatters, linters, validators, CLIs,
   tests and scripts through the core runner/broker;
4. small native first-party platform adapters for PTY, window/webview,
   accessibility, file dialogs, keychain references, and process cleanup.

A signed package does not move to a more privileged tier. Native shared
libraries and arbitrary install scripts are not public extension payloads.

Supervised tier-3 processes normally retain the operating-system account's
authority unless a separately evidenced sandbox/container profile confines them.
Automexia limits what it supplies and authorizes, but exact executable/argv/cwd/
environment, deadlines, output ceilings, cancellation, and descendant cleanup
must not be presented as a complete process sandbox.

Opening a workspace permits bounded viewing/editing in restricted mode, not
execution. Language servers, workspace-defined executable paths, tools, tasks,
hooks, providers, credentials, network, debug adapters, and production actions
remain disabled until exact trust and capability decisions pass. Workspace
trust is necessary but not sufficient; grants bind principal, version/digest,
capability, workspace/path/target, profile, operation, quotas, review generation,
and expiry.

Future tool or script execution uses a typed `ScriptRunIntent`. It binds the
saved document revision/digest, tool identity/version, exact arguments,
validated working directory, Environment Capsule revision, selected target,
opaque secret references, operation/risk class, numeric limits, capability,
immutable review/plan generation, production confirmation, and expiry. Every
material input is revalidated immediately before execution. There is no shell
command string, shell evaluation, hidden elevation, input to an existing PTY, or
implicit Enter. The initial run path accepts saved canonical revisions only.

The feature is lazy and optional. Installing Studio does not create a webview,
scan a workspace, start a language server, or launch a tool. Disable, uninstall,
crash, and failed update return to terminal-only behavior, native shell editors,
CP1 completion, and current first-party extensions. Cleanup removes only exact
Automexia-owned package/cache/grant/recovery generations and never user files,
shell profiles, provider state, external tools, or credentials.

## Alternatives

- Integrate the file editor directly into DevOps/SRE: rejected because it
  couples a reusable editing foundation to one domain and makes terminal-only
  DevOps inherit unnecessary weight and platform risk.
- Put a full IDE/editor in the terminal core: rejected because it widens the
  mandatory startup, dependency, browser, resource, attack, and maintenance
  footprint and creates competing tool/provider ownership.
- Open a separate IDE window or application: retained only as an explicit
  external-editor action; it does not provide the integrated terminal, target,
  review, and document experience intended here.
- Draw a file editor in the terminal grid: rejected because PTY cells are not a
  document revision, IME, accessibility, multi-view, conflict, or save protocol.
- Adopt Monaco without comparison: rejected. Its richer desktop surface remains
  valuable, but official mobile non-support and wider service/worker integration
  require measurement against the smaller CodeMirror target.
- Embed Neovim as Automexia's document owner: rejected. Neovim remains a
  supported terminal/external-editor workflow, but its buffers, process, RPC,
  plugins, and UI would become a second product authority.
- Build the editor, syntax ecosystem, language intelligence, WebAssembly runtime,
  or cryptographic verifier from scratch: rejected; use maintained specialist
  components behind Automexia-owned policy and lifecycle contracts.
- Give Studio or extensions direct filesystem/process/network/credential/PTY
  access: rejected; typed proposals re-enter core brokers and current policy.
- Make Studio wait for a public marketplace: rejected for first-party slices.
  Public third-party delivery still waits for ADR 0029.

## Required acceptance and verification

No CodeMirror, Monaco, Wry, WebView2/WebKit integration, Studio crate/package,
document service, webview host, LSP/DAP broker, language server, manifest
extension, UI/configuration/shortcut, file-write capability, script-run path, or
activation flag is authorized by this proposed record.

Before acceptance, AS0 must provide:

- exact editor/host/dependency versions, features, licenses, provenance,
  advisories, MSRV, transitive dependencies, unsafe/authority footprint, update
  owners, binary/package/build/startup/resource measurements, and rollback;
- inspected native Windows, macOS, Linux X11, and Linux Wayland proof for the
  actual Automexia window/renderer stack, including focus, IME, accessibility,
  scale, resize, z-order, GPU, crash, cleanup, and packaging;
- a versioned machine contract with numeric document, IPC, webview, LSP,
  process, queue, cache, storage, recovery, timeout, retry, crash, concurrency,
  and shutdown ceilings plus hostile fixtures and mutation owners;
- explicit replacement/supersession wording for the exact ADR 0003 file-write,
  process, network, webview, or third-party capability slices requested;
- terminal-only baseline and disable/uninstall/rollback proof.

Acceptance authorizes only the reviewed next source slice. File write, process,
network, credential, provider, production, remote, debug, collaboration, AI, or
third-party authority still requires its existing protected review and native
evidence. ADR 0023 continues to own M6 automation activation, and ADR 0029
continues to own public signed/sandboxed packages.

Verification follows
[Automation Studio testing](../AUTOMATION-STUDIO-TESTING.md): deterministic
unit/model/property/fuzz/mutation tests, malicious workspace/webview/LSP/tool
corpora, native UI/IME/accessibility/package runs, resource and long-session
measurement, exact run-review revalidation, secret canaries, process-tree
cleanup, recovery, and install/update/disable/uninstall/rollback. Claims remain
limited to the exact operating systems, architectures, tools, versions, targets,
durations, and configurations that ran.

Primary references:
[CodeMirror](https://codemirror.net/),
[Monaco Editor](https://github.com/microsoft/monaco-editor),
[Wry](https://github.com/tauri-apps/wry),
[LSP 3.18](https://microsoft.github.io/language-server-protocol/specifications/lsp/3.18/specification/),
[DAP](https://microsoft.github.io/debug-adapter-protocol/),
[VS Code Workspace Trust](https://code.visualstudio.com/api/extension-guides/workspace-trust),
[secure WebView2 guidance](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security),
[Wasmtime interruption](https://docs.wasmtime.dev/examples-interrupting-wasm.html),
and [Wasmtime resource limiting](https://docs.wasmtime.dev/api/wasmtime/trait.ResourceLimiter.html).

## Consequences

- Users get one integrated Automexia workspace without losing the terminal-first
  fallback or being forced to install an IDE-sized feature.
- Domain extensions remain reusable, independently releasable, and least-
  privilege; the convenience Pack does not become a capability principal.
- Core code grows several generic brokers, but each centralizes an authority that
  would otherwise be duplicated across editors, languages, and domains.
- The editor UI may use web technology, but web content is never trusted with
  native authority or canonical persistence.
- Linux Wayland and native accessibility/IME evidence become explicit blockers,
  not inferred from another desktop or window system.
- First-party delivery can proceed independently of the public D7 marketplace,
  while third-party packages keep the stricter signed/sandboxed boundary.
- The saved-revision rule deliberately postpones convenient execution of unsaved
  text until immutable temporary-artifact ownership is proven.
- More advanced debugging, remote/mobile, collaboration, and AI workflows remain
  separate decisions rather than silently inheriting Studio approval.
- The first stable terminal release remains independent of Studio. A minimal
  evidenced Studio precedes a video-editing product, while video research and
  later Studio phases may advance independently through shared generic services.
