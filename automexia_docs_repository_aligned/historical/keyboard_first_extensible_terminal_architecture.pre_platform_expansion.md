# Rust-First Keyboard-Only Extensible Terminal Platform
## Product Architecture, Security Model, Extension System, DevOps Access Layer, and Implementation Specification

**Status:** Architecture and implementation blueprint
**Audience:** Founders, terminal/runtime engineers, security engineers, extension-platform engineers, DevOps/platform engineers, product engineers
**Primary goal:** Define a robust Rust-first architecture for a fast terminal emulator that is useful by itself, has no Termius-style graphical management UI or clickable-button workflow, and can become a broader terminal platform through secure extensions and add-ons.

---

# 1. Executive Summary

The product should not be designed as “Ghostty plus Termius features” or as “a terminal with plugins.”

It should be designed as a **keyboard-first terminal platform** with three major layers:

1. A **high-performance trusted terminal core**.
2. A **secure capability platform** that exposes commands, resources, sessions, authentication, process execution, storage, and terminal-native text surfaces.
3. A **sandboxed extension ecosystem** where major feature families—DevOps, databases, AI, Git, security, cloud operations, observability, file transfer, and future categories—can be added without bloating or destabilizing the terminal core.

The terminal must remain excellent even when every extension is disabled.

The DevOps capabilities discussed in this design are therefore **not the main product architecture**. They are an example of what can be built on top of the platform. The same extension mechanisms should allow entirely different categories of functionality to exist later.

The recommended high-level architecture is:

```text
                         TERMINAL PLATFORM

┌──────────────────────────────────────────────────────────────────────┐
│                           Terminal Frontend                                │
│                                                                      │
│ GPU Renderer   Keyboard Input   Command Mode   Terminal Text Surfaces     │
└───────────────────────────────┬──────────────────────────────────────┘
                                │
                              IPC/API
                                │
┌───────────────────────────────▼──────────────────────────────────────┐
│                          Trusted Core                                │
│                                                                      │
│ Sessions  Commands  Resources  Config  Security  Extension Manager   │
│ Credential Broker  Process Broker  Storage  Updates  Policy Engine   │
└─────────────┬─────────────────┬───────────────────┬──────────────────┘
              │                 │                   │
              ▼                 ▼                   ▼
      ┌──────────────┐   ┌───────────────┐   ┌────────────────────┐
      │ Session Host │   │ Extension Host│   │ Credential Broker  │
      │              │   │               │   │                    │
      │ PTY/ConPTY   │   │ Wasmtime/WASM │   │ SSH Agents         │
      │ VT Engine    │   │ WASI          │   │ OS Secure Storage  │
      │ Scrollback   │   │ Permissions   │   │ Vault Providers    │
      │ Shell        │   │ Resource Caps │   │ SSO Sessions       │
      └──────┬───────┘   └───────┬───────┘   └─────────┬──────────┘
             │                   │                     │
             ▼                   ▼                     ▼
       bash/zsh/etc.         Extensions             Providers
                                │
              ┌─────────────────┼─────────────────────────────┐
              ▼                 ▼                             ▼
           DevOps            Database                         AI
              │
   SSH / K8s / Cloud / Teleport / Docker / Vaults / etc.
```

The most important architectural rule is:

> **Extensions extend capabilities; they do not own the terminal.**


## 1.1 Non-Negotiable Rust Implementation Contract

Rust is not an optional implementation suggestion for this project. It is the primary implementation language and the architectural contracts in this document assume a Rust workspace.

The default implementation baseline is:

```text
Language                  Rust
Workspace                 Cargo workspace
Rust edition              2024
Async/control plane       Tokio
Extension runtime         Wasmtime
Extension ABI             WebAssembly Component Model + WIT
GPU abstraction           Renderer trait; wgpu as initial backend
Terminal/VT engine        Rust trait; libghostty-vt may sit behind isolated FFI
Metadata storage          SQLite
Secret storage            OS secure store / external credential providers
IPC                       versioned typed protocol over local OS IPC
Serialization             explicit versioned schema; never raw Rust ABI
Observability             tracing-style structured events with redaction
Fuzz/property testing     cargo-fuzz/proptest-style targets where applicable
```

The architecture must preserve a **safe-Rust default**:

```rust
#![forbid(unsafe_code)]
```

for ordinary domain crates. `unsafe` is permitted only in narrow, explicitly designated boundary crates such as:

```text
platform-pty-unix
platform-conpty-windows
terminal-vt-ffi
renderer-platform
os-integration
```

Every unsafe block must document:

1. the invariant being relied on,
2. ownership/lifetime assumptions,
3. thread-safety assumptions,
4. what validates input before crossing the boundary,
5. how the invariant is tested.

No extension is loaded through the Rust ABI or `dlopen`/`LoadLibrary` into the trusted process. Sandboxed extensions target the WIT/Component Model contract instead.

## 1.2 Rust Is the Platform Language, Not the Extension Lock-In

The trusted implementation is Rust-first, but third-party extensions should not be forced to share the application's Rust ABI.

```text
Trusted platform                  Rust
             ↓
Versioned WIT interfaces
             ↓
WebAssembly Component Model
             ↓
Rust / Go / C / C++ / other guest languages as tooling matures
```

First-party extensions may also be written in Rust and compiled to components, but they should exercise the same public extension interfaces wherever possible. Privileged first-party modules that must cross a stronger trust boundary must be explicitly classified and reviewed.

## 1.3 No Graphical Management Layer

The Rust frontend is a terminal renderer and keyboard input surface, not a Termius-style GUI application shell. It must not introduce a parallel graphical application model with buttons, cards, sidebar navigation, host tiles, graphical forms, or mouse-only actions.

First-party interaction primitives are textual:

```rust
pub enum TextSurfaceRequest {
    CommandSearch(CommandSearchRequest),
    SelectList(SelectListRequest),
    ShowTable(ShowTableRequest),
    ShowTree(ShowTreeRequest),
    PromptText(PromptTextRequest),
    PromptSecret(PromptSecretRequest),
    ConfirmKeypress(ConfirmKeypressRequest),
    ShowProgress(ProgressRequest),
    ShowStatus(StatusRequest),
}
```

These are rendered into terminal-owned text surfaces and controlled by keys. Extensions provide data and actions; they do not draw arbitrary graphical controls.

The VT parser, PTY/ConPTY lifecycle, renderer, input path, security policy, credential mediation, extension runtime, update verification, and crash recovery belong to the trusted platform.

Cloud providers, DevOps workflows, databases, AI, Kubernetes tooling, Git workflows, and similar functionality belong outside that trusted core.

---

# 2. Product Vision

The product should feel like an extremely fast native terminal first.

A user should be able to:

```text
global shortcut
      ↓
terminal appears instantly
      ↓
type normally into zsh/bash/fish/PowerShell/etc.
```

Nothing about the extension platform should interfere with ordinary terminal usage.

When the user wants advanced capabilities, they should be one keyboard gesture away:

```text
Ctrl+Space / Cmd+K / custom shortcut
                ↓
        Universal Command Layer
                ↓
   search commands, environments,
   servers, clouds, clusters, tools,
   extensions, workflows, sessions
```

The terminal becomes a place from which the user can quickly access:

- local shells,
- SSH servers,
- bastions,
- Teleport clusters,
- AWS accounts and roles,
- Azure subscriptions,
- GCP projects,
- Kubernetes clusters and namespaces,
- containers,
- remote development environments,
- databases,
- secret providers,
- Git repositories,
- CI systems,
- infrastructure tools,
- custom company environments,
- extension-defined resources.

The product is therefore not merely a terminal emulator.

A better conceptual description is:

> **A fast terminal runtime with a programmable, secure, keyboard-native capability layer.**

---

# 3. Product Principles

The following principles should be treated as architectural requirements, not just preferences.

## 3.1 The terminal must stand on its own

If the extension system is broken or disabled, the user must still have an excellent terminal emulator.

The core terminal must support:

- local shell sessions,
- native PTY/ConPTY,
- excellent rendering,
- Unicode,
- scrollback,
- tabs/splits if desired,
- keybindings,
- terminal search,
- clipboard,
- shell compatibility,
- configuration,
- crash recovery,
- stable updates.

Extensions must not be required for basic terminal operation.

---

## 3.2 Keyboard-only product interaction; no Termius-style GUI

The product must not evolve into a graphical remote-host manager. This is a non-negotiable product constraint.

The terminal window itself may be created by the operating system and rendered with GPU acceleration, but **all product interaction inside it is text-first and keyboard-driven**. There are no clickable connection cards, toolbar buttons, graphical settings pages, mouse-driven server browsers, or Termius-style management panels.

The primary interaction model is:

- normal shell typing,
- configurable keyboard shortcuts,
- a keyboard-invoked command mode,
- fuzzy text search,
- keyboard-controlled lists/tables/trees rendered as terminal text surfaces,
- typed prompts,
- explicit keypress confirmations,
- inline status/progress text,
- commands exposed through both keybindings and a textual command namespace.

Example:

```text
Ctrl+Space
    ↓
command mode opens inside the terminal surface
    ↓
> prod api
    ↓
↑/↓ or j/k to select
Enter to execute
Esc to cancel
```

There are **no clickable buttons** in this flow. Every operation must be reachable without leaving the keyboard.

### Mouse compatibility is not product navigation

The terminal emulator still must implement terminal mouse-reporting protocols because applications such as Vim, Neovim, tmux, htop, text editors, TUIs, and remote programs may legitimately request mouse events through VT/xterm protocols. That compatibility belongs to the terminal engine.

However:

```text
mouse support for programs running inside the terminal  ✅
mouse-driven management of this terminal product       ❌
clickable host cards / connect buttons                  ❌
graphical settings/dashboard                            ❌
keyboard commands / text selectors                      ✅
```

The project should therefore use the term **terminal text surface** or **command mode**, not graphical panel, widget, button, or dashboard, for first-party product interaction.

---

## 3.3 The hot path must stay small

The latency-sensitive path should look approximately like this:

```text
Keyboard
   ↓
Trusted Input Manager
   ↓
Session IPC
   ↓
PTY/ConPTY
```

And terminal output:

```text
PTY/ConPTY
   ↓
VT Engine
   ↓
Terminal State
   ↓
Damage Tracker
   ↓
GPU Renderer
```

The following must **not** be inserted into the hot path:

- cloud SDKs,
- extension callbacks,
- AI,
- database access,
- remote API calls,
- credential providers,
- package managers,
- resource discovery,
- network requests.

---

## 3.4 Extensions must be isolated

An extension crash should not crash the terminal.

A malicious or buggy extension should not automatically be able to:

- read all keystrokes,
- read arbitrary terminal output,
- access raw credentials,
- open arbitrary files,
- access the entire network,
- spawn arbitrary processes,
- inject code into the renderer,
- disable security checks.

---

## 3.5 Credentials should be delegated

The terminal should avoid becoming a password manager.

Whenever possible, credentials should remain in:

- SSH agents,
- 1Password,
- Bitwarden,
- KeePassXC,
- OpenBao,
- HashiCorp Vault,
- OS keychains,
- cloud identity systems,
- browser/device-based SSO,
- enterprise identity providers.

The terminal should **broker access**, not own every secret.

---

## 3.6 Prefer temporary identity over permanent secrets

The system should prefer:

```text
identity
   ↓
authentication
   ↓
short-lived credential/session
   ↓
access
   ↓
automatic expiry
```

over:

```text
store permanent password/key forever
```

This applies especially to:

- AWS,
- Azure,
- GCP,
- Kubernetes,
- Teleport,
- SSH certificates,
- enterprise remote access.

---

## 3.7 Extensions contribute resources and actions

The core should not contain every domain concept.

Instead, extensions contribute generic concepts:

```text
Resource
Action
Command
AccessPlan
AuthenticationProvider
CredentialProvider
Workflow
```

For example:

```text
ssh.host
aws.account
kubernetes.cluster
postgres.database
docker.container
teleport.node
```

are resource types contributed by extensions.

---

# 4. Major Subsystems

The project should be decomposed into the following major systems:

```text
Terminal Platform
│
├── Terminal Engine
├── Session Runtime
├── GPU Renderer
├── Input Manager
├── Command System
├── Terminal Text-Surface System
├── Resource Platform
├── Access Plan Engine
├── Credential Broker
├── Process Broker
├── Security/Policy Engine
├── Extension Runtime
├── Extension SDK
├── Storage
├── Configuration
├── IPC
├── Update System
├── Diagnostics
├── Telemetry/Privacy Controls
└── First-Party Extensions
```

Each subsystem should have a narrow contract.

---

# 5. Trusted Core vs Extension Boundary

This boundary is critical.

## 5.1 Trusted core responsibilities

The trusted core should own:

- application startup,
- global shortcuts,
- terminal renderer,
- font rendering,
- VT parsing or VT-engine integration,
- terminal state,
- input encoding,
- PTY/ConPTY,
- process lifecycle,
- shell launching,
- session persistence,
- IPC protocol,
- command registry,
- resource registry,
- permission enforcement,
- credential broker,
- secure local storage abstraction,
- process broker,
- extension runtime,
- extension package verification,
- updater,
- crash recovery,
- diagnostics,
- security policy,
- extension API compatibility.

---

## 5.2 Extension responsibilities

Extensions should own domain-specific functionality.

Examples:

### DevOps extension
- SSH destination discovery,
- Kubernetes resources,
- AWS resources,
- Azure resources,
- GCP resources,
- Teleport resources,
- Docker resources,
- cloud workflows,
- environment switching,
- port forwarding,
- bastion orchestration,
- vault/provider integrations,
- AccessPlan creation.

### Database extension
- PostgreSQL resources,
- MySQL resources,
- Redis resources,
- saved database endpoints,
- connection workflows,
- database-specific command actions.

### Git extension
- repository resources,
- branch actions,
- worktree actions,
- Git hosting integrations.

### AI extension
- explain output,
- generate commands,
- diagnose failures,
- workflow assistance,
- opt-in context access.

The core should not need to know the details of these domains.

---

# 6. Recommended Process Architecture

A multi-process architecture is strongly recommended.

Do not build a single giant process containing:

```text
renderer
PTY
extensions
cloud SDKs
SSH
vaults
AI
database integrations
```

A better model is:

```text
┌──────────────────────┐
│     terminal-frontend      │
│                      │
│ renderer             │
│ keyboard/input       │
│ text surfaces             │
│ window management    │
└──────────┬───────────┘
           │ IPC
           ▼
┌──────────────────────┐
│        termd         │
│                      │
│ session supervisor   │
│ command registry     │
│ resource registry    │
│ config               │
│ policy               │
│ extension manager    │
└────┬─────────┬───────┘
     │         │
     │         └───────────────────────────┐
     ▼                                     ▼
┌──────────────┐                    ┌───────────────┐
│ session-host │                    │ extension-host│
│              │                    │               │
│ PTY/ConPTY   │                    │ Wasmtime      │
│ VT engine    │                    │ WASM          │
│ scrollback   │                    │ WASI          │
│ shell proc   │                    │ permissions   │
└──────────────┘                    └───────────────┘

              ┌───────────────────────────────┐
              │ Credential / Access Services  │
              │                               │
              │ agents / keychains / vaults   │
              │ SSO sessions / temp creds     │
              └───────────────────────────────┘
```

---

# 7. Why Session Hosts Matter

The PTY and shell session should not necessarily die just because the frontend process dies.

A session host can own:

```text
SessionHost
│
├── PTY or ConPTY
├── shell process
├── VT parser / terminal state
├── scrollback
├── environment
├── process metadata
└── session metadata
```

This enables:

```text
frontend crash
   ↓
session continues

frontend restarts
   ↓
reattach
```

It also enables optional persistent behavior:

```text
close terminal window
        ↓
session remains detached

later
        ↓
reattach
```

This provides some of the resilience users like in tools such as tmux, while preserving a modern terminal application architecture.

---

# 8. Terminal Engine

Terminal emulation is security-sensitive and complex.

The terminal engine must handle:

- UTF-8 decoding,
- VT state machine,
- ESC sequences,
- CSI,
- OSC,
- DCS,
- APC,
- cursor state,
- alternate screen,
- terminal modes,
- scroll regions,
- bracketed paste,
- mouse reporting,
- focus reporting,
- hyperlinks,
- colors,
- synchronized output,
- modern keyboard protocols,
- graphics protocols if supported,
- line wrapping,
- reflow,
- grapheme clusters,
- combining characters,
- wide characters,
- terminal titles,
- notifications,
- clipboard control policy.

The data flow should be:

```text
bytes
  ↓
decoder
  ↓
VT parser
  ↓
terminal state
  ↓
screen/grid
  ↓
damage information
```

The renderer should consume terminal state. It should not be tightly coupled to the parser.

---

# 9. Reusing an Existing VT Engine

A reasonable strategy is to wrap an existing mature terminal engine behind your own interface.

For example:

```text
TerminalEngine trait/interface
        │
        ├── GhosttyVtAdapter
        └── FutureAlternative
```

This is better than scattering library-specific calls throughout the codebase.

Your own interface might expose concepts such as:

```text
TerminalEngine
├── feed(bytes)
├── resize(cols, rows)
├── get_damage()
├── snapshot()
├── encode_key(event)
├── encode_mouse(event)
├── paste(text)
├── scroll(...)
└── query_mode(...)
```

This preserves the ability to change implementation later.

---

# 10. PTY and ConPTY

## 10.1 Unix

On Linux and macOS:

```text
terminal
   │
 master PTY
   │
 pseudo-terminal
   │
 slave PTY
   │
shell/application
```

The terminal owns the PTY master.

The shell sees a terminal through the slave.

---

## 10.2 Windows

Use ConPTY rather than trying to emulate Windows console behavior independently.

The abstraction should be:

```text
PtyBackend
├── UnixPty
├── MacPty
└── WindowsConPty
```

Everything above this layer should operate on a common interface.

Example:

```text
PtyBackend
├── spawn(command, env, size)
├── resize(cols, rows)
├── read()
├── write(bytes)
├── signal(...)
├── wait()
└── terminate()
```

Windows read/write servicing must be designed carefully to avoid blocking and deadlock scenarios.

---

# 11. Renderer Architecture

The renderer must be independent from the terminal parser.

Recommended pipeline:

```text
PTY
 ↓
VT engine
 ↓
Terminal Grid
 ↓
Damage Tracker
 ↓
Text Shaping / Glyph Selection
 ↓
GPU Buffers
 ↓
Frame
```

A cell may conceptually contain:

```text
Cell {
    grapheme
    foreground
    background
    attributes
    width
    hyperlink
}
```

The renderer handles:

- background rectangles,
- glyph shaping,
- glyph atlas,
- cursor,
- selection,
- decorations,
- underline styles,
- hyperlinks,
- images if supported,
- text-surface compositing.

---

# 12. Damage Tracking

Do not redraw the entire terminal unnecessarily.

Example:

```text
100 x 40 terminal
only row 31 changed
```

The renderer should only update what is necessary.

The rendering system should support:

- row damage,
- cell-range damage,
- cursor damage,
- scroll damage,
- full-screen invalidation only when necessary,
- glyph atlas invalidation,
- resize invalidation.

This becomes important under large terminal output.

---

# 13. Backpressure and Output Flooding

A remote process can produce huge amounts of output.

The architecture needs bounded queues.

```text
PTY
 ↓
bounded read queue
 ↓
VT parser
 ↓
bounded terminal state
 ↓
damage coalescing
 ↓
renderer
```

The renderer should not attempt to render every intermediate update.

If the screen changes 2,000 times between frames:

```text
keep latest coherent terminal state
```

rather than rendering 2,000 states.

---

# 14. Scrollback

Scrollback must be bounded.

Do not model it as an infinitely growing vector.

A good conceptual structure:

```text
ScrollbackStore
├── recent hot lines
├── compact older lines
├── style table
├── hyperlink table
├── image references
└── limits
```

Configurable limits should exist for:

- number of lines,
- total memory,
- hyperlink metadata,
- graphics payloads,
- OSC payloads,
- clipboard payloads.

Malicious remote output must not be able to consume unlimited memory.

---

# 15. Input Architecture

Keyboard input must be trusted-core functionality.

Recommended flow:

```text
Keyboard
   ↓
Input Manager
   ↓
Keybinding Resolver
   ├── terminal command
   ├── extension command
   └── PTY input
```

Ordinary extensions should not globally receive every keypress.

This avoids accidentally building a plugin keylogger platform.

Extensions should normally receive events such as:

```text
command invoked
resource selected
session changed
```

rather than raw keyboard streams.

Highly privileged raw-input observation should require explicit elevated permission.

---

# 16. Keep the User's Shell

The terminal should not replace zsh/bash/fish/PowerShell/nushell.

Normal input should go directly to the shell.

Example:

```text
type:
git status
```

This goes to the shell.

A special shortcut opens the terminal platform command layer:

```text
Ctrl+Space
```

Then:

```text
> Connect
> Switch Session
> Open Resource
> Install Extension
> AWS Login
```

Escape returns immediately to the normal shell.

This preserves compatibility with existing CLI workflows.

---

# 17. Core Command System

Every platform action should be represented as a command.

Examples:

```text
terminal.new
terminal.close
terminal.split
terminal.search
terminal.zoomIn
terminal.session.next
terminal.session.detach

commandMode.open

resource.search
resource.connect

extension.install
extension.disable
extension.reload

security.permissions

devops.ssh.connect
devops.aws.login
devops.kubernetes.switch
```

The command system powers:

- keyboard shortcuts,
- command mode,
- extension actions,
- automation,
- discoverability,
- scripting,
- testing.

A command should have:

```text
Command {
    id
    title
    category
    description
    availability predicate
    handler
    required permissions
}
```

---

# 18. Terminal-Native Text Surface System

A keyboard-first terminal still needs structured interaction.

The trusted core should provide declarative keyboard-controlled text interaction primitives.

Examples:

```text
select_from_list()
prompt_text()
prompt_secret()
show_text_table()
show_text_tree()
confirm_keypress()
show_text_progress()
show_status_message()
show_text_details()
showDiff()
```

Extensions should provide data, not custom rendering logic.

Example picker:

```text
┌─────────────────────────────────────────────────────┐
│ Connect                                              │
├─────────────────────────────────────────────────────┤
│ > prod                                               │
│                                                      │
│  prod-api-01     SSH            eu-west-1            │
│  prod-api-02     SSH            eu-west-1            │
│  production      AWS            account              │
│  prod-eu         Kubernetes     cluster              │
└─────────────────────────────────────────────────────┘
```

Benefits:

- consistent keyboard behavior,
- one theme system,
- accessibility,
- no extension-specific graphical toolkit,
- easier security auditing,
- predictable performance,
- no arbitrary plugin drawing in the renderer.

---

# 19. Terminal Text-Surface Rendering

Text surfaces should be composited separately:

```text
GPU Frame
├── terminal background
├── terminal grid
├── decorations
├── cursor
├── terminal graphics
└── trusted text-surface layer
    ├── command mode
    ├── resource selector
    ├── permission prompt
    ├── auth prompt
    └── progress text
```

The shell session can continue running underneath.

---

# 20. Universal Resource Model

This is one of the most important abstractions.

The core should not need separate hard-coded models for:

- SSH host,
- AWS account,
- Kubernetes cluster,
- database,
- Docker container,
- Teleport node.

Instead:

```text
Resource
```

A conceptual model:

```text
Resource {
    id
    kind
    name
    provider
    locator
    labels
    metadata
    capabilities
    parent
}
```

Examples:

```text
ssh.host
aws.account
aws.role
aws.instance
azure.subscription
gcp.project
kubernetes.cluster
kubernetes.namespace
docker.context
docker.container
postgres.database
redis.instance
teleport.node
```

Extensions define new kinds.

---

# 21. Resource Index

Resources should be discovered asynchronously and normalized into a local index.

Example:

```text
~/.ssh/config
     ↓
SSH Resource Provider
     ↓
Resource {
    kind = ssh.host
    name = prod-api
}
     ↓
Resource Index
```

Other sources:

- AWS config/profile files,
- Azure CLI state,
- gcloud configs,
- kubeconfig,
- Docker contexts,
- Teleport profiles,
- enterprise configuration,
- extension-defined sources.

The resource selector searches the local index instantly.

It should not query cloud APIs on every keystroke.

---

# 22. Universal Search Experience

The user should be able to press one shortcut and search across infrastructure.

Example:

```text
Ctrl+P

> production
```

Potential results:

```text
production-api-01       SSH Host
production-admin        AWS Role
production-eu           Kubernetes Cluster
production-db           PostgreSQL
production-bastion      Teleport
production-container    Docker
```

This is more powerful than a saved-host manager because it normalizes many infrastructure types into one searchable model.

---

# 23. Resource Actions

Resources expose actions.

Example:

```text
SSH Host
├── Connect
├── Run Command
├── Open Tunnel
└── Copy File
```

```text
AWS Account
├── Login
├── Open Shell
├── Switch Role
└── Refresh Resources
```

```text
Kubernetes Cluster
├── Connect
├── Set Context
├── Open Shell
├── Port Forward
└── Browse Namespaces
```

The generic model is:

```text
Resource
   ↓
Actions
```

An extension contributes both resources and actions.

---

# 24. Access Plan

Connections are often multi-step.

Therefore define an abstraction such as:

```text
AccessPlan
```

Example:

```text
AccessPlan: production-api

1. resolve destination
2. resolve user identity
3. check active authentication session
4. authenticate if required
5. obtain short-lived credential
6. resolve bastion/proxy
7. validate host trust
8. start SSH
9. create terminal session
10. track credential expiry
```

Another:

```text
AWS SSO
   ↓
assume role
   ↓
obtain Kubernetes token
   ↓
connect to EKS
   ↓
launch shell with context
```

The terminal platform executes plans.

Extensions define domain-specific plans.

This avoids hard-coding cloud logic in the core.

---

# 25. Access Plan Model

A plan could contain typed steps.

For example:

```text
AccessPlan {
    resource_id
    steps[]
    resulting_context
}
```

Potential step types:

```text
ResolveIdentity
EnsureAuthentication
AcquireCredential
StartProxy
VerifyHost
SetEnvironment
LaunchProcess
AttachPty
SetSessionContext
RegisterExpiry
```

The engine can support:

- cancellation,
- rollback,
- retry policy,
- progress reporting,
- sensitive-step redaction,
- audit events.

---

# 26. Session Context

Each session should have structured context.

Example:

```text
SessionContext
├── local machine
├── shell
├── cwd
├── Git repository
├── SSH destination
├── cloud account
├── cloud role
├── cloud region
├── Kubernetes cluster
├── namespace
├── container
├── identity
└── credential expiration
```

Extensions can query the context through permissioned APIs.

This enables:

- safer production indicators,
- context-aware commands,
- resource-specific actions,
- session recovery,
- AI opt-in context,
- better history search.

---

# 27. Production Context Safety

The terminal can surface high-risk context without blocking normal work.

Example prompt/status:

```text
PROD | AWS: payments-prod | K8s: prod-eu | ns: payments
```

Potential safeguards:

- explicit production label,
- environment color/theme hint,
- optional confirmation for destructive extension actions,
- policy-based restrictions,
- session-specific command warnings.

Do not make the terminal intrusive; make safety context available and predictable.

---

# 28. Credential Broker

The credential broker should be trusted-core functionality.

Extensions should not independently retrieve secrets.

Correct:

```text
Extension
    ↓
Credential Broker
    ↓
Provider Adapter
```

Wrong:

```text
Extension → Bitwarden secret
Extension → 1Password secret
Extension → OpenBao token
Extension → AWS keys
```

The broker centralizes:

- permission checks,
- credential selection,
- identity resolution,
- MFA orchestration,
- temporary materialization,
- expiration,
- revocation,
- audit,
- secure caching,
- secret redaction.

---

# 29. Credential Handling Philosophy

The preferred model is:

```text
Extension
   ↓
requests ability to authenticate
   ↓
Credential Broker
   ↓
returns opaque credential capability
```

rather than:

```text
Extension
   ↓
receives password/private key/token bytes
```

Use opaque handles where possible.

Example:

```text
CredentialHandle #8437
type: aws-session
expires: 17:43
```

Then:

```text
ProcessBroker.launch(
    command = "aws",
    credential_handles = [#8437]
)
```

The broker injects what the process requires.

The extension does not necessarily see the secret material.

---

# 30. Credential Provider Interface

A conceptual provider API:

```text
CredentialProvider
├── capabilities()
├── status()
├── authenticate()
├── listIdentities()
├── resolve()
├── sign()
├── issueTemporaryCredential()
├── refresh()
├── revoke()
└── lock()
```

Providers may support only some capabilities.

---

# 31. SSH Agent Integration

For SSH, prefer the agent model.

Architecture:

```text
OpenSSH
   ↓
SSH_AUTH_SOCK
   ↓
SSH Agent
   ↓
1Password / Bitwarden / KeePassXC / system agent
   ↓
sign operation
```

The terminal does not need the private key bytes.

A generic abstraction might expose:

```text
SshSigner
├── listIdentities()
├── sign(public_key_id, payload)
└── confirmUse(...)
```

The best integration is often to let OpenSSH speak directly to the configured agent.

---

# 32. Built-In Local Secure Storage

If the product allows “store locally,” this should initially mean:

```text
OS-provided secure storage
```

not a custom password vault.

Recommended mapping:

```text
macOS   → Keychain
Windows → Credential Manager / secure OS APIs
Linux   → Secret Service / desktop keyring integration
```

The application's ordinary database should store a reference, not the secret.

Example:

```text
credential_provider = "os-keychain"
credential_ref = "terminal:ssh:prod-api"
```

---

# 33. Normal Application Database

Use a normal local database for metadata, not secrets.

A practical choice is SQLite.

Potential tables:

```text
resources
resource_labels
resource_actions_cache
sessions
session_metadata
aliases
extensions
extension_state
command_metadata
history_metadata
provider_metadata
auth_session_metadata
settings_index
```

Do not store:

```text
passwords
SSH private keys
vault master keys
long-lived cloud secrets
MFA seeds
```

unless there is an explicitly designed, audited secure-storage feature.

---

# 34. OpenBao

OpenBao should be a **provider/integration**, not a mandatory architectural dependency.

Correct:

```text
Credential Broker
├── 1Password
├── Bitwarden
├── KeePassXC
├── OpenBao
├── HashiCorp Vault
├── OS Keychain
└── Cloud Identity
```

OpenBao is particularly useful for enterprise environments where teams want:

- dynamic secrets,
- temporary credentials,
- short-lived SSH certificates,
- PKI,
- centralized policies,
- audit trails.

Example flow:

```text
User authenticates to OpenBao
        ↓
Terminal requests SSH certificate
        ↓
OpenBao signs user's public key
        ↓
short-lived certificate
        ↓
OpenSSH connects
        ↓
certificate expires automatically
```

The terminal should orchestrate this, not become OpenBao.

---

# 35. Termius-Like Credential Features

The product may eventually offer Termius-like convenience:

- saved host profiles,
- saved username references,
- credential references,
- SSH identity selection,
- certificate references,
- host grouping,
- connection aliases,
- tunnels,
- jump hosts.

But credentials should be stored via a secure provider.

Example resource:

```text
prod-api

Host:        10.10.20.31
User:        ubuntu
Port:        22
Jump Host:   prod-bastion
Credential:  1Password/company-prod
Tags:        aws, production, payments
```

The profile belongs in the application's resource database.

The secret remains in the vault/agent.

---

# 36. Cloud Authentication Model

Cloud authentication should be modeled as sessions, not permanent passwords.

Use:

```text
Identity
   ↓
Authentication Session
   ↓
Temporary Credential
   ↓
Expiration
```

not:

```text
username/password
```

Session metadata can include:

```text
provider
identity
account
role
region
expires_at
refresh_method
```

---

# 37. AWS

Initial strategy:

- detect AWS CLI,
- discover configured profiles,
- understand IAM Identity Center profiles,
- invoke official AWS authentication flows,
- observe successful session establishment,
- create session context,
- provide resource/action integration.

Example:

```text
User selects:
AWS / production-admin
        ↓
session expired
        ↓
run official AWS SSO login flow
        ↓
browser/device authentication
        ↓
temporary credentials available
        ↓
launch shell with AWS context
```

Do not reimplement AWS SSO protocol first.

Your value is orchestration.

---

# 38. Azure

Likewise:

```text
resource selected
      ↓
check Azure CLI session
      ↓
authenticate through official flow if required
      ↓
select subscription
      ↓
launch session context
```

Use the vendor CLI before building a native OAuth stack.

---

# 39. GCP

Use the same pattern:

```text
gcloud identity/session
      ↓
project selection
      ↓
temporary application/CLI credentials
      ↓
session context
```

Again, the product should orchestrate existing proven identity flows.

---

# 40. Kubernetes

Kubernetes should be modeled around:

- kubeconfig resources,
- contexts,
- clusters,
- users,
- namespaces,
- external exec credential providers.

Do not assume the terminal can store all Kubernetes credentials itself.

Example:

```text
Kubernetes cluster
      ↓
kubeconfig context
      ↓
exec credential provider
      ↓
AWS/GCP/Azure/enterprise auth
      ↓
temporary token
      ↓
kubectl
```

A session can expose:

```text
cluster = prod-eu
namespace = payments
```

while leaving credential mechanics to standard tooling.

---

# 41. Teleport

Initial Teleport integration should orchestrate official tooling.

Example:

```text
Teleport resource
      ↓
check session
      ↓
tsh login
      ↓
MFA/browser if required
      ↓
short-lived credentials
      ↓
tsh/openSSH proxy
      ↓
session host
```

Do not implement the whole Teleport authentication protocol on day one.

---

# 42. SSH

SSH should initially rely on OpenSSH.

The terminal should orchestrate:

- host discovery,
- aliases,
- jump hosts,
- ProxyCommand/ProxyJump,
- agent selection,
- known_hosts,
- host certificates,
- tunnels,
- port forwarding,
- connection context,
- environment setup.

Avoid building a new SSH implementation unless there is a compelling later requirement.

---

# 43. SSH Host Verification

Do not trade security for convenience.

Unknown host:

```text
explicit first-use confirmation
```

Changed host key:

```text
strong stop/warning
```

Respect:

- known_hosts,
- certificate authorities,
- host certificates,
- user configuration,
- enterprise policies.

Never silently accept a changed host key.

---

# 44. Agent Forwarding

Agent forwarding should be explicit, not globally enabled.

Resource policy example:

```text
prod-api:
    agent_forwarding = false

dev-workstation:
    agent_forwarding = true
```

The access plan should know whether forwarding is allowed for that target.

---

# 45. DevOps Extension Architecture

The DevOps extension should be a first-party extension built on generic platform APIs.

```text
DevOps Extension
│
├── Resource Providers
│   ├── SSH
│   ├── AWS
│   ├── Azure
│   ├── GCP
│   ├── Kubernetes
│   ├── Docker
│   ├── Teleport
│   └── enterprise providers
│
├── Actions
│   ├── Connect
│   ├── Login
│   ├── Switch Context
│   ├── Port Forward
│   ├── Run Command
│   └── Refresh
│
├── Access Plan Builders
│
├── Credential Provider Bridges
│
├── Commands
│
└── Workflows
```

It should not bypass the security/capability brokers.

---

# 46. Example DevOps Flow: SSH

User:

```text
Ctrl+P
> prod api
```

System:

```text
Resource Index
   ↓
prod-api-04
kind: ssh.host
   ↓
DevOps Extension
   ↓
build AccessPlan
   ↓
Credential Broker
   ↓
SSH agent available
   ↓
OpenSSH
   ↓
Session Host
   ↓
PTY
   ↓
Terminal
```

User experience:

```text
Ctrl+P
prod api
Enter
```

Everything else is handled underneath.

---

# 47. Example DevOps Flow: AWS to Kubernetes

User selects:

```text
payments-prod
kind: kubernetes.cluster
cloud: AWS
account: production
region: eu-west-1
namespace: payments
```

Plan:

```text
AWS SSO session valid?
        │
        ├── yes ──────────────┐
        │                     │
        └── no                │
            ↓                 │
        AWS login             │
            ↓                 │
        temporary session     │
            └─────────────────┘
                    ↓
        kubeconfig exec credential
                    ↓
        Kubernetes authentication
                    ↓
        terminal session context
                    ↓
        shell
```

Prompt/status may display:

```text
PROD / AWS:production / K8s:payments-prod / ns:payments
```

---

# 48. Example DevOps Flow: Teleport

```text
Resource: prod-bastion
        ↓
Teleport session expired
        ↓
official tsh login
        ↓
MFA/browser
        ↓
short-lived certificate/session
        ↓
proxy connection
        ↓
SSH
        ↓
PTY
```

The terminal does not hold permanent Teleport credentials.

---

# 49. Example DevOps Flow: OpenBao SSH Certificate

```text
User selects secure-prod-host
        ↓
resolve OpenBao identity
        ↓
authenticate to OpenBao
        ↓
request signed SSH cert
        ↓
certificate TTL = 30m
        ↓
launch OpenSSH
        ↓
session established
        ↓
credential expiry tracked
```

If the certificate expires during a long-lived session, the product can surface this in session metadata without necessarily interrupting the current connection.

---

# 50. Extension Runtime

Extensions should not be loaded as arbitrary in-process native libraries.

Recommended default:

```text
WebAssembly Component
      ↓
Wasmtime
      ↓
WASI / Component Model
      ↓
Capability-Restricted Host APIs
```

Benefits:

- process isolation,
- memory isolation,
- portable extensions,
- resource controls,
- explicit capabilities,
- easier crash containment,
- language flexibility.

---

# 51. Extension Manifest

Extensions should be declarative first.

Example:

```toml
id = "acme.devops"
name = "DevOps"
version = "1.4.0"
api = "1"

runtime = "wasm-component"
entry = "devops.wasm"

[activation]
events = [
    "onCommand:devops.connect",
    "onResource:ssh.host",
    "onResource:kubernetes.cluster"
]

[permissions]
resources.read = true
network.connect = true
process.execute = true
credentials.request = true

[contributes]
commands = [
    "devops.connect",
    "devops.aws.login",
    "devops.kubernetes.switch"
]
```

The manifest should also define:

- package metadata,
- extension API compatibility,
- publisher identity,
- requested capabilities,
- contributed resource kinds,
- contributed commands,
- activation events,
- configuration schema,
- optional migrations.

---

# 52. Lazy Extension Activation

Extensions should not load during normal terminal startup unless absolutely necessary.

Bad:

```text
terminal starts
  ↓
load every extension
  ↓
run discovery
  ↓
initialize network
  ↓
terminal becomes slow
```

Correct:

```text
terminal starts
  ↓
trusted core only
  ↓
user invokes devops.connect
  ↓
activate DevOps extension
```

Resource discovery can be scheduled after startup or activated based on relevant signals.

---

# 53. Extension Capability Model

Permissions should be fine-grained.

Examples:

```text
terminal.session.readMetadata
terminal.output.read
terminal.input.write

resources.read
resources.write

filesystem.read
filesystem.write

network.connect

process.execute

credentials.request
credentials.sign

clipboard.read
clipboard.write

notifications.show
```

More sensitive:

```text
terminal.rawInput.observe
terminal.rawOutput.observe
credentials.materialize
process.executeUnrestricted
filesystem.unrestricted
network.unrestricted
```

These should require explicit elevated consent and may be disallowed for marketplace extensions.

---

# 54. No Global Keylogging

Ordinary extensions must not receive every keystroke.

Default:

```text
keyboard
   ↓
trusted input manager
   ↓
keybinding resolver
   ↓
shell OR command
```

Extension receives:

```text
command: devops.connect
```

not:

```text
key: p
key: a
key: s
key: s
...
```

Raw input observation should be an extraordinary permission.

---

# 55. Terminal Output Permissions

Terminal output may contain:

- passwords,
- API tokens,
- internal hostnames,
- customer data,
- private repository names,
- database output,
- company infrastructure.

Therefore an AI extension or other plugin should not automatically receive output.

Possible permissions:

```text
terminal.output.read.selection
terminal.output.read.currentCommand
terminal.output.read.visible
terminal.output.read.session
```

Each level should be clearly communicated.

---

# 56. Extension Process Execution

Extensions should never receive unrestricted `exec()` by default.

Use a trusted Process Broker.

```text
Extension
   ↓
ProcessBroker.execute(LaunchSpec)
   ↓
permission check
   ↓
environment filtering
   ↓
credential injection
   ↓
spawn process
```

A launch specification could contain:

```text
LaunchSpec {
    executable
    argv[]
    cwd
    environment_additions
    credential_handles[]
    stdin_mode
    stdout_mode
    timeout
    network_policy
}
```

Prefer structured arguments.

Do not build shell strings from untrusted input.

---

# 57. Credential Materialization

Sometimes an external program requires a token in:

- environment variable,
- file,
- stdin,
- command-line parameter.

The broker should support tightly scoped temporary materialization.

Example:

```text
CredentialHandle
      ↓
temporary environment variable
      ↓
child process only
      ↓
process exits
      ↓
environment destroyed
```

Or:

```text
temporary file
   ↓
restrict permissions
   ↓
launch child
   ↓
delete file
```

Raw material should be exposed only when unavoidable.

---

# 58. Extension Isolation

Use defense in depth:

```text
OS process isolation
        +
WASM sandbox
        +
capability permissions
        +
resource limits
```

Per-extension limits can include:

- maximum memory,
- CPU/fuel budget,
- maximum tasks,
- network destinations,
- filesystem mounts,
- event queue size,
- execution timeout.

---

# 59. One Extension Host per Extension

A strong model is:

```text
extension-host -- devops.wasm
extension-host -- ai.wasm
extension-host -- database.wasm
```

Advantages:

- crash attribution,
- separate memory limits,
- separate CPU limits,
- easy termination,
- improved fault containment,
- cleaner security policy.

Processes are created lazily.

---

# 60. Native Extensions

Native extensions should be exceptional.

If supported:

- run in a separate process,
- require publisher signing,
- require explicit elevated trust,
- use restricted IPC,
- never `dlopen()` marketplace code into the core.

Possible trust hierarchy:

```text
Trusted Core
   ↓
First-Party Privileged Extensions
   ↓
Signed Sandboxed Extensions
   ↓
Local Development Extensions
   ↓
Native Elevated Extensions
```

Native does not automatically mean more trusted; it means more dangerous and therefore needs stronger control.

---

# 61. WebAssembly Component API

Define extension interfaces with stable WIT-like concepts.

Possible modules:

```text
terminal:commands
terminal:sessions
terminal:resources
terminal:text-surface
terminal:process
terminal:credentials
terminal:storage
terminal:network
terminal:notifications
terminal:context
```

Avoid exposing internal Rust structures.

The host implementation may change while the API remains stable.

---

# 62. Extension SDK

Treat the extension SDK as a product.

Potential tools:

```bash
term ext new
term ext dev
term ext test
term ext package
term ext publish
```

SDK contents:

```text
sdk/
├── wit/
├── rust/
├── go/
├── examples/
├── test-runtime/
├── mock-host/
└── packaging/
```

Developers should not need to understand renderer internals or IPC internals.

---

# 63. Extension API Versioning

The extension API must be versioned independently from the application.

Example:

```text
Terminal App 7.4
Extension API 2
```

Interfaces can be versioned individually:

```text
terminal:commands@1
terminal:resources@1
terminal:sessions@1
terminal:credentials@1
```

Do not break the ecosystem every time the internal implementation changes.

---

# 64. Extension Package Format

Possible structure:

```text
devops.termext
├── manifest.toml
├── extension.wasm
├── assets/
├── schemas/
├── migrations/
└── signature
```

Installation flow:

```text
download
   ↓
verify package hash
   ↓
verify publisher signature
   ↓
validate manifest
   ↓
compare permissions
   ↓
user consent if required
   ↓
install atomically
```

---

# 65. Permission Changes on Update

An extension update that asks for new permissions must not silently receive them.

Example:

```text
DevOps 2.4 requests:

+ clipboard.read

Approve update permissions?
```

If declined, the previous version can remain installed.

This prevents permission escalation through updates.

---

# 66. Extension Signing

Extension signing should eventually include:

- publisher identity,
- package signature,
- content hash,
- version,
- API compatibility,
- manifest hash,
- optional transparency log,
- revocation support.

Enterprise deployments may need:

- allowlists,
- blocklists,
- private registries,
- pinned versions,
- publisher restrictions,
- permission policies.

---

# 67. Extension Event Bus

Extensions need asynchronous events.

Examples:

```text
onSessionStarted
onSessionEnded
onCwdChanged
onCommandStarted
onCommandFinished

onResourceAdded
onResourceRemoved
onResourceUpdated

onAuthenticationExpired
onCredentialExpiring

onNetworkChanged
onConfigurationChanged
```

Events must never block the terminal hot path.

Use:

```text
core
  ↓
bounded queue
  ↓
extension host
```

not synchronous extension callbacks from the renderer/session loop.

---

# 68. Shell Integration

Optional shell integration can expose:

- current working directory,
- command boundaries,
- command start,
- command end,
- exit status,
- prompt boundaries,
- shell identity.

Then features can include:

- jump between prompts,
- rerun failed command,
- copy only command output,
- annotate commands,
- AI explain selected command output,
- searchable semantic history.

If shell integration is missing:

```text
terminal still functions normally
```

---

# 69. Semantic History

Traditional history is just command text.

The platform can optionally model:

```text
CommandRecord {
    command_text
    cwd
    started_at
    duration
    exit_code
    session_id
    resource_context
    environment_context
}
```

Then queries such as:

```text
failed kubectl production yesterday
```

become possible.

Security requirements:

- opt-out,
- redaction,
- history suppression,
- private session mode,
- configurable retention,
- no secret capture by default.

---

# 70. Resource Discovery

Resource discovery runs asynchronously.

Examples:

```text
SSH Provider
   reads ~/.ssh/config

Kubernetes Provider
   reads kubeconfig

AWS Provider
   reads AWS config and cached profile metadata

Docker Provider
   reads Docker contexts

Teleport Provider
   reads local Teleport profiles
```

Results are normalized into the Resource Index.

Network enumeration should happen on an explicit refresh or controlled background cadence, not in the keystroke search path.

---

# 71. Search Index

Search must be local and fast.

Potential indexes:

- resource name,
- aliases,
- tags,
- provider,
- resource kind,
- environment,
- project,
- account,
- region,
- recency,
- usage frequency.

Ranking can combine:

```text
fuzzy match
+ recency
+ frequency
+ exact alias
+ context similarity
```

The initial implementation can remain simple.

---

# 72. Configuration Model

Configuration should be layered:

```text
defaults
   ↓
system policy
   ↓
user config
   ↓
workspace config
   ↓
profile
   ↓
session override
```

Use a human-readable format such as TOML.

Example:

```toml
[terminal]
shell = "/bin/zsh"
scrollback_lines = 100000

[keybindings]
"ctrl+space" = "commandMode.open"

[extensions.devops]
enabled = true
```

Never place raw secrets in ordinary config.

---

# 73. Extension Configuration Namespaces

Extensions should only own their namespace.

Example:

```toml
[extensions.devops.aws]
refresh_interval = "5m"

[extensions.ai]
provider = "example"
```

An extension should not be able to write:

```text
terminal.security.disable = true
```

---

# 74. IPC

Because the product is multi-process, IPC is a major protocol.

Connections may include:

```text
terminal-frontend ↔ termd
termd ↔ session-host
termd ↔ extension-host
termd ↔ credential services
```

The protocol should support:

- versioning,
- request IDs,
- responses,
- errors,
- cancellation,
- streams,
- backpressure,
- capability negotiation,
- deadlines/timeouts,
- reconnection.

---

# 75. High-Frequency vs Control IPC

Separate high-frequency terminal traffic conceptually from lower-frequency control traffic.

Control:

```text
create session
close session
rename session
resize
set metadata
extension activation
configuration updates
```

High-frequency:

```text
PTY output
terminal state changes
keyboard input
```

Do not let extension-management traffic block terminal I/O.

If profiling later shows serialization costs are significant, shared-memory buffers may be introduced selectively.

Do not start with unnecessary complexity.

---

# 76. Storage

A simple architecture:

```text
state.db
│
├── resources
├── aliases
├── sessions
├── extension metadata
├── provider metadata
├── command metadata
└── cached discovery data
```

Secure secrets:

```text
OS keychain / vault / agent / provider
```

Never confuse the two.

---

# 77. Security Model

The security model should assume:

- remote terminal output is hostile,
- extensions may be malicious,
- cloud APIs may return untrusted data,
- config files may be malformed,
- plugin packages may be tampered with,
- shell output may contain secrets,
- user commands may expose sensitive values,
- child processes may behave unexpectedly.

---

# 78. Secret-Safe Types

Introduce types such as:

```text
Secret<T>
SensitiveString
CredentialHandle
Redacted<T>
```

These types should:

- not implement normal debug printing,
- redact logs,
- avoid accidental serialization,
- zero memory where practical,
- have explicit materialization APIs.

Avoid:

```text
log("env = " + env)
```

because it will eventually expose secrets.

---

# 79. Logging

Logs should be structured.

Examples:

```text
event=session_started
session_id=...
resource_id=...
```

not giant unstructured dumps.

Sensitive fields should be automatically redacted.

Crash reports must not contain:

- terminal contents by default,
- credential material,
- environment variables,
- command output,
- command history,
- raw config secrets.

---

# 80. Remote Terminal Output Is Untrusted

A malicious remote host can send:

- huge escape sequences,
- malformed UTF-8,
- oversized OSC payloads,
- graphics payloads,
- clipboard control sequences,
- misleading hyperlinks,
- notification spam,
- terminal title abuse.

Set limits and policies.

Fuzz:

- VT parser,
- OSC parser,
- DCS parser,
- Unicode handling,
- graphics parser,
- clipboard handling,
- shell-integration parser.

---

# 81. Clipboard Security

Remote applications should not be able to silently exfiltrate clipboard data.

Have explicit policy for:

```text
OSC clipboard read
OSC clipboard write
```

Possible default:

- allow clipboard writes only with policy,
- deny clipboard reads unless explicitly enabled,
- show permission or status for sensitive operations.

Extensions need separate clipboard permissions.

---

# 82. URL/Hyperlink Security

Hyperlinks in terminal output are untrusted.

The platform should:

- normalize/validate schemes,
- avoid invisible spoofing,
- expose a keyboard-invoked destination preview/status action before opening,
- block dangerous custom schemes unless allowed,
- avoid automatic execution.

---

# 83. Process Broker Security

Process execution should enforce:

- explicit executable,
- structured argument list,
- environment filtering,
- cwd validation,
- permission checks,
- optional executable allowlists,
- no implicit shell unless requested.

Avoid:

```text
shell -c "<user data>"
```

where possible.

---

# 84. Network Capability Security

Extensions should request network permissions.

Possible models:

```text
network.connect = true
```

later evolving into:

```text
network:
  - api.github.com:443
  - api.example.com:443
```

Enterprise policy may restrict extensions to approved destinations.

---

# 85. Filesystem Capability Security

A WASM extension should not see the entire filesystem by default.

Prefer capability mounts such as:

```text
read:
  ~/.ssh/config

read:
  ~/.kube/config
```

rather than:

```text
filesystem = unrestricted
```

If a provider needs broader access, request it explicitly.

---

# 86. Resource-Specific Permissions

The model should be able to evolve toward policies such as:

```text
database-extension:
    resources.read:
        staging/*
        development/*

    resources.read:
        production/* = denied
```

This is especially valuable for enterprise environments.

---

# 87. Update Security

The terminal updater itself is part of the trusted computing base.

Requirements:

- signed releases,
- verified update manifests,
- rollback protection where appropriate,
- atomic installation,
- rollback on failed launch,
- staged migration,
- extension compatibility checks.

Never allow extensions to replace core binaries.

---

# 88. Safe Mode

Always support a safe mode.

Example:

```bash
terminal --safe-mode
```

Safe mode should:

- disable third-party extensions,
- use safe/default configuration,
- disable extension networking,
- avoid optional startup hooks,
- preserve ability to inspect diagnostics.

Also:

```bash
terminal --disable-extension publisher.name
```

---

# 89. Diagnostics

Provide:

```bash
terminal doctor
```

Example output:

```text
Terminal Core
  ✓ PTY
  ✓ renderer
  ✓ shell
  ✓ font engine

Extensions
  ✓ DevOps 1.4.2
  ✓ Git 1.1.0

Credentials
  ✓ SSH agent detected
  ✓ 1Password agent
  ✗ Bitwarden agent
  ✓ OS secure storage

DevOps
  ✓ OpenSSH
  ✓ kubectl
  ✓ AWS CLI
  ✓ Azure CLI
  ✓ gcloud
  ✓ tsh

Security
  ✓ extension signatures
  ✓ secure storage
  ✓ safe mode available
```

Diagnostics must not expose secrets.

---

# 90. Failure Isolation

Design for failure intentionally.

Test:

```text
kill extension-host
kill terminal-frontend
kill session-host
kill network
lock password manager
expire AWS session
change SSH host key
disconnect VPN
kill OpenBao
send malformed terminal output
produce huge output
```

Expected behavior:

- extension crash → terminal continues,
- frontend crash → session survives if architecture permits,
- auth provider unavailable → clear recoverable error,
- network disappears → session-specific behavior,
- host key changes → stop connection,
- malformed output → parser survives.

---

# 91. Performance Goals

Performance is part of product identity.

Measure:

- cold startup,
- warm startup,
- global shortcut to visible frame,
- key-to-frame latency,
- PTY throughput,
- VT parse throughput,
- GPU frame time,
- memory per session,
- scrollback memory,
- command mode latency,
- resource search latency,
- extension activation latency,
- idle CPU,
- idle RAM.

Create explicit budgets.

Do not rely on subjective “feels fast.”

---

# 92. Frontend/Render Thread Policy

The frontend/render thread should do very little.

It may:

- receive input,
- update lightweight frontend state,
- submit render work,
- dispatch IPC.

It should not:

- call cloud APIs,
- query large databases,
- run extensions,
- perform MFA,
- run SSH authentication,
- scan fonts synchronously,
- parse certificates synchronously,
- block on disk.

---

# 93. Extension Performance Rules

Extensions:

- activate lazily,
- run out-of-process,
- cannot block rendering,
- use bounded event queues,
- have memory limits,
- have execution budgets,
- are killable,
- are observable through diagnostics.

If a plugin becomes slow:

```text
plugin becomes slow
      ↓
plugin gets throttled/terminated
      ↓
terminal remains responsive
```

---

# 94. Mandatory Rust Language and Runtime Architecture

The project is primarily implemented in Rust. This is an architectural decision, not a temporary recommendation.

## 94.1 Workspace shape

Use one Cargo workspace with small crates that express trust and dependency boundaries:

```text
workspace/
├── apps/
│   ├── terminal/                 # terminal frontend: window, renderer, keyboard
│   ├── termd/                    # trusted coordinator/session supervisor
│   ├── session-host/             # owns PTY/ConPTY + child process + terminal state
│   └── extension-host/           # isolated Wasmtime host process
│
├── crates/
│   ├── core-types/               # IDs, wire-safe domain types; forbid unsafe
│   ├── ipc-protocol/             # versioned message schema
│   ├── ipc-transport/            # Unix sockets / Windows named pipes
│   ├── command-core/             # command registry/execution contracts
│   ├── text-surface/             # keyboard-only command/search/select surfaces
│   ├── session-core/             # session state machine
│   ├── pty-core/                 # safe PTY trait
│   ├── pty-unix/                 # Unix PTY boundary; narrow unsafe if required
│   ├── conpty-windows/           # Windows ConPTY boundary
│   ├── terminal-engine/          # safe VT abstraction
│   ├── terminal-vt-ffi/          # optional libghostty-vt FFI boundary
│   ├── render-model/             # renderer-independent frame/damage model
│   ├── renderer/                 # Renderer trait
│   ├── renderer-wgpu/            # initial wgpu backend
│   ├── input-core/               # keymap/input routing
│   ├── resource-core/            # generic Resource/Action model
│   ├── access-plan/              # typed plan graph + executor
│   ├── credential-core/          # handles/provider traits; never logs secrets
│   ├── credential-broker/        # policy + lifetime + provider mediation
│   ├── process-broker/           # argv-based controlled process launch
│   ├── security-policy/          # capabilities and authorization
│   ├── extension-api/            # host-side WIT bindings
│   ├── extension-runtime/        # Wasmtime sandbox and limits
│   ├── extension-manager/        # install/activate/update/disable lifecycle
│   ├── storage/                  # SQLite metadata repositories/migrations
│   ├── config/                   # layered typed configuration
│   ├── diagnostics/              # doctor/health/structured diagnostics
│   ├── observability/            # tracing/redaction/audit envelopes
│   ├── updater/                  # signed update verification
│   └── platform/                 # OS feature abstractions
│
├── extensions/
│   ├── devops/                   # first-party extension
│   ├── git/
│   ├── database/
│   └── examples/
│
├── sdk/
│   ├── wit/                      # public source of truth for extension ABI
│   ├── rust/                     # ergonomic Rust guest SDK
│   └── examples/
│
├── tests/
│   ├── conformance/
│   ├── integration/
│   ├── security/
│   ├── failure/
│   ├── fuzz/
│   └── performance/
│
└── docs/
    ├── architecture/
    ├── security/
    ├── extensions/
    └── adr/
```

## 94.2 Dependency direction

Crates must follow a directed dependency graph. Domain types do not depend on runtime services; services depend on domain contracts; applications compose services.

```text
apps
 ↓
orchestration/services
 ↓
commands  sessions  resources  access  credentials  extensions
 ↓
security-policy + core-types
 ↓
platform boundary crates
```

Avoid cycles such as:

```text
sessions → extensions → credentials → sessions
```

If two crates need each other, extract the shared contract into a lower-level crate.

## 94.3 Typed IDs and domain separation

Never model unrelated identifiers as plain `String` values.

```rust
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct SessionId(uuid::Uuid);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ResourceId(uuid::Uuid);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ExtensionId(String);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CommandId(String);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct CredentialHandle(uuid::Uuid);
```

Do not make `SessionId` implicitly convertible to `ResourceId` or vice versa.

## 94.4 Secret types

Raw secrets must use types that are non-printable by default.

```rust
pub struct Secret<T>(T);

impl<T> std::fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Secret([REDACTED])")
    }
}
```

Prefer proven secrecy/zeroization abstractions where appropriate. The design rule is more important than a specific helper crate:

```text
secret value     → cannot accidentally Debug/Display
credential       → represented as CredentialHandle when possible
materialization  → narrow scoped operation
logs/crashes     → redacted by construction
```

## 94.5 Tokio ownership

Tokio owns the asynchronous control plane:

- local IPC,
- extension-host communication,
- provider orchestration,
- resource discovery,
- file watchers,
- network requests performed by trusted services,
- child-process supervision where async fits,
- credential expiry/refresh timers,
- background indexing.

Tokio must **not** become the terminal render scheduler. The latency-sensitive terminal path should use dedicated ownership and bounded channels.

Recommended execution domains:

```text
frontend/render thread
    keyboard dispatch
    frame preparation/submission

session-host runtime
    PTY reads/writes
    child lifecycle
    VT parsing/state ownership

Tokio control runtime
    IPC/orchestration/provider work

extension-host runtime
    Wasmtime async execution + budgets
```

Never `await` cloud authentication, SQLite maintenance, extension execution, or network I/O on the render path.

## 94.6 State ownership

Use single-owner state where possible instead of broad `Arc<Mutex<Everything>>` designs.

Preferred:

```text
SessionActor owns SessionState
ResourceIndex service owns resource index writes
CredentialBroker owns credential/session lifecycle
ExtensionHost owns one extension instance
Renderer owns GPU objects
```

Communicate through typed messages. Use shared synchronization only for truly shared, low-contention data.

## 94.7 Cancellation and timeouts

Every potentially blocking operation must support cancellation or a defined timeout:

```rust
pub struct OperationContext {
    pub request_id: RequestId,
    pub cancellation: CancellationToken,
    pub deadline: Option<Instant>,
}
```

This is required for:

- provider logins,
- vault operations,
- process launches,
- access-plan steps,
- extension calls,
- remote discovery,
- update downloads.

## 94.8 Error architecture

Core libraries use typed errors; binary/application edges may wrap them for diagnostics.

```rust
pub enum CredentialError {
    Locked,
    PermissionDenied,
    AuthenticationRequired,
    Expired,
    ProviderUnavailable,
    UnsupportedOperation,
}

pub enum AccessError {
    ResourceUnavailable,
    AuthenticationFailed,
    HostVerificationFailed,
    ProxyFailed,
    ProcessLaunchFailed,
    Cancelled,
}
```

Do not erase every error into a single string before policy/recovery logic has had a chance to inspect it.

## 94.9 `unsafe` policy

Default crate policy:

```rust
#![forbid(unsafe_code)]
```

Only boundary crates may relax this. CI should fail if `unsafe` appears elsewhere.

Boundary crates must expose safe wrappers. No raw pointer, native handle, or FFI lifetime escapes into domain code.

## 94.10 Extension ABI

Do not expose the Rust ABI to plugins. Rust ABI stability is not the extension contract.

```text
Rust core
   ↓
WIT interface definitions
   ↓
Wasmtime host bindings
   ↓
WebAssembly Components
```

The WIT packages are separately versioned, for example:

```text
terminal:commands@1
terminal:resources@1
terminal:sessions@1
terminal:text-surface@1
terminal:credentials@1
terminal:process@1
terminal:network@1
```

## 94.11 Process launch API

Extensions never receive a raw `std::process::Command` capability. They submit a validated launch specification:

```rust
pub struct LaunchSpec {
    pub executable: ExecutableRef,
    pub args: Vec<OsString>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<EnvAssignment>,
    pub credential_handles: Vec<CredentialHandle>,
    pub stdin_mode: StdinMode,
    pub stdout_mode: StdoutMode,
    pub timeout: Option<Duration>,
}
```

The process broker validates permissions, resolves credentials narrowly, sanitizes inherited environment variables, and launches without constructing shell command strings.

## 94.12 Rust API stability

Internal Rust APIs may evolve rapidly. Public stability exists at:

1. user configuration schema,
2. IPC protocol compatibility boundaries that require it,
3. WIT extension API versions,
4. extension package manifest format,
5. persisted database migration contracts.

Do not freeze internal crates merely because they are Rust libraries in the same workspace.

---

# 95. Renderer Backend Strategy

Possible initial strategy:

```text
Renderer trait
   ↓
WgpuRenderer
```

Backend mapping:

```text
macOS   → Metal
Windows → Direct3D 12
Linux   → Vulkan
```

Wrap the implementation so a future specialized renderer is possible.

Example:

```text
Renderer
├── WgpuRenderer
├── NativeMetalRenderer
└── FutureBackend
```

Only add native specialization if profiling demonstrates a real need.

---

# 96. First-Party vs Third-Party Extensions

First-party extensions can be granted broader capabilities because they are shipped and audited with the product.

Example trust levels:

```text
Level 0: Core
Level 1: First-party privileged extension
Level 2: Signed sandboxed marketplace extension
Level 3: Local development extension
Level 4: Native elevated extension
```

The DevOps extension may begin as Level 1 because it interacts with:

- authentication,
- process execution,
- network connections,
- secure providers.

---

# 97. Extension Marketplace Timing

Do not build a marketplace first.

Before marketplace:

1. stabilize extension API,
2. build sandbox,
3. build permission model,
4. build signing,
5. build package verification,
6. build safe mode,
7. build diagnostics,
8. build extension testing tools,
9. prove upgrade compatibility.

Marketplace distribution comes afterward.

---

# 98. Rust Cargo Workspace Layout

The repository should be a Cargo workspace. The following is the baseline layout; names may evolve, but the trust/dependency boundaries should remain:

```text
terminal/
│
├── apps/
│   ├── terminal/
│   ├── termd/
│   ├── session-host/
│   └── extension-host/
│
├── crates/
│   ├── core-types/
│   ├── ipc/
│   ├── commands/
│   ├── config/
│   ├── platform/
│   ├── pty/
│   ├── terminal-engine/
│   ├── terminal-engine-adapter/
│   ├── renderer/
│   ├── fonts/
│   ├── sessions/
│   ├── resources/
│   ├── access-plan/
│   ├── security/
│   ├── credentials/
│   ├── process-broker/
│   ├── extension-runtime/
│   ├── extension-api/
│   ├── storage/
│   ├── updater/
│   └── diagnostics/
│
├── extensions/
│   ├── devops/
│   ├── git/
│   ├── database/
│   └── examples/
│
├── sdk/
│   ├── wit/
│   ├── rust/
│   ├── go/
│   └── examples/
│
├── tests/
│   ├── vt/
│   ├── pty/
│   ├── rendering/
│   ├── security/
│   ├── extensions/
│   ├── credentials/
│   ├── integration/
│   └── performance/
│
└── docs/
    ├── architecture/
    └── adr/
```

---

# 99. Core Data Types

Keep shared types stable.

Examples:

```text
SessionId
ResourceId
ExtensionId
CommandId
CredentialHandle
AuthSessionId
AccessPlanId
ProviderId
```

Do not allow arbitrary untyped strings everywhere.

Typed identifiers reduce accidental mixups.

---

# 100. Resource Type Example

Conceptual Rust-like structure:

```text
Resource {
    id: ResourceId,
    kind: ResourceKind,
    name: String,
    provider: ProviderId,
    parent: Option<ResourceId>,
    labels: Map<String, String>,
    metadata: StructuredMetadata,
    actions: Vec<ActionDescriptor>,
}
```

The core stores generic structured metadata.

Extensions interpret their own domain-specific metadata.

---

# 101. Action Descriptor

```text
ActionDescriptor {
    id: String,
    title: String,
    command: CommandId,
    required_permissions: Vec<Capability>,
    availability: ActionCondition,
}
```

This allows generic resource text surface.

---

# 102. Access Plan Example Schema

```text
AccessPlan {
    id: AccessPlanId,
    target: ResourceId,
    steps: Vec<AccessStep>,
    expected_context: SessionContext,
}
```

Possible step:

```text
AccessStep::EnsureAuth {
    provider,
    identity,
}
```

```text
AccessStep::AcquireCredential {
    request,
    output_handle,
}
```

```text
AccessStep::LaunchProcess {
    launch_spec,
}
```

---

# 103. Credential Request Example

```text
CredentialRequest {
    purpose
    target_resource
    credential_kind
    identity_hint
    required_capabilities
    minimum_ttl
}
```

The broker chooses a compatible provider.

---

# 104. Provider Capability Matching

Example:

```text
Request:
    need SSH signing capability
```

Available:

```text
1Password SSH agent → supported
Bitwarden SSH agent → supported
OpenBao SSH cert    → alternative strategy
OS Keychain         → not a signer by itself
```

The access plan can choose the appropriate path.

---

# 105. DevOps Provider Strategy

Start by supporting standards and existing tools.

Priority:

```text
1. OpenSSH / SSH agent
2. ~/.ssh/config
3. kubeconfig
4. AWS CLI
5. Azure CLI
6. gcloud
7. Docker CLI/contexts
8. Teleport tsh
9. 1Password agent
10. Bitwarden agent
11. KeePassXC agent compatibility
12. OpenBao
13. HashiCorp Vault
```

The exact order can change based on target users.

The architectural point is to integrate widely used interfaces before implementing custom proprietary behavior.

---

# 106. What “Local Credentials” Should Mean

A user may choose:

```text
credential source:
    local
```

That should mean:

```text
local secure storage provider
```

not:

```text
plain configuration file
```

The terminal may expose a unified credential selector:

```text
Credential:
  • 1Password / Engineering / prod-ssh
  • Bitwarden / Production / prod-ssh
  • OS Keychain / prod-api
  • OpenBao / ssh/prod
```

The selected value stored in the resource profile is only a reference.

---

# 107. Provider Independence

Never design:

```text
DevOps extension depends directly on OpenBao
```

Prefer:

```text
DevOps Extension
      ↓
Credential Broker API
      ↓
Provider abstraction
```

This allows the same SSH resource to work with:

- 1Password today,
- Bitwarden tomorrow,
- OpenBao at work,
- OS Keychain on a personal machine.

---

# 108. Extension-Specific Credential Providers

Third-party providers should be possible eventually.

However:

- provider extensions must be heavily sandboxed,
- raw-secret access should be minimized,
- signing/temporary credentials are preferable,
- sensitive provider permissions may require first-party review or enterprise policy approval.

---

# 109. User Experience: Fast Connection

Example:

```text
global shortcut
     ↓
terminal appears
     ↓
Ctrl+P
     ↓
type: prod
```

Results:

```text
prod-api-01        SSH
prod-admin         AWS
prod-eu            Kubernetes
prod-db            PostgreSQL
prod-bastion       Teleport
```

Enter on SSH:

```text
resolve host
  ↓
resolve identity
  ↓
agent available
  ↓
host trust verified
  ↓
connect
```

No manual command construction required.

---

# 110. User Experience: Still a Normal Terminal

The advanced system must never prevent:

```bash
ssh user@example.com
kubectl get pods
aws sts get-caller-identity
git status
python app.py
```

from working exactly as they normally would.

The platform adds shortcuts and orchestration, but it does not take ownership of normal shell behavior.

---

# 111. Extension Example: Database

The architecture should also support something unrelated to DevOps.

```text
Database Extension
│
├── postgres.database resources
├── mysql.database resources
├── redis.instance resources
├── actions
│   ├── connect
│   ├── open client
│   ├── tunnel
│   └── inspect
└── credential requests
```

It can reuse:

- Resource model,
- Credential Broker,
- Process Broker,
- AccessPlan,
- terminal text surfaces.

This proves the platform architecture is generic.

---

# 112. Extension Example: AI

An AI extension can contribute:

```text
ai.explainSelection
ai.explainLastCommand
ai.suggestFix
ai.generateCommand
```

But it only receives terminal context with user-approved permissions.

Possible model:

```text
user invokes "Explain last command"
      ↓
core identifies last command output
      ↓
permission granted?
      ↓
provide bounded selected context
      ↓
AI extension
```

Do not continuously stream all terminal output to AI by default.

---

# 113. Extension Example: Git

Git extension:

```text
git.repository resources
```

Actions:

```text
open worktree
switch branch
create worktree
show repository status
open shell in repo
```

Again, no core changes required.

---

# 114. Extension Example: Security

A security extension might contribute:

- certificate inspection,
- SSH trust diagnostics,
- secret scanning of selected output,
- infrastructure context inspection,
- policy checks.

It must still operate through permissions.

---

# 115. Core Stability Rule

A new feature should enter the core only if it meets a high bar:

- required by the terminal itself,
- security boundary,
- performance hot path,
- universal primitive,
- impossible to implement safely as extension.

If not, it should probably be an extension.

---

# 116. ADR Strategy

This project should use Architecture Decision Records.

Suggested ADRs:

| ADR | Topic |
|---|---|
| 0001 | Product architecture and trusted-core boundary |
| 0002 | Primary implementation language |
| 0003 | Terminal/VT engine |
| 0004 | PTY/ConPTY abstraction |
| 0005 | Renderer architecture |
| 0006 | Multi-process session model |
| 0007 | IPC protocol |
| 0008 | Command system |
| 0009 | Terminal text surface model |
| 0010 | Universal Resource model |
| 0011 | AccessPlan model |
| 0012 | Extension runtime |
| 0013 | WebAssembly/Component Model |
| 0014 | Extension permissions |
| 0015 | Process broker |
| 0016 | Credential broker |
| 0017 | Local secret storage |
| 0018 | Extension signing |
| 0019 | Update security |
| 0020 | Storage/database model |
| 0021 | DevOps extension |
| 0022 | Provider API |
| 0023 | SSH architecture |
| 0024 | OpenBao integration |
| 0025 | Cloud authentication |
| 0026 | Telemetry/privacy |
| 0027 | Safe mode/recovery |
| 0028 | Session persistence |
| 0029 | Extension marketplace |
| 0030 | Native extension policy |

Each ADR should contain:

```text
Title
Status
Context
Decision
Alternatives Considered
Security Impact
Operational Impact
Consequences
Migration/Compatibility
```

---

# 117. Why OpenBao Needs Its Own ADR

Accepting an SSH design does not automatically authorize OpenBao integration.

SSH ADR:

```text
Can the product establish SSH sessions?
What SSH implementation is used?
How is host verification handled?
How are agents handled?
```

OpenBao ADR:

```text
Can OpenBao issue/manage credentials?
What authentication methods are supported?
Can it issue SSH certs?
Can it issue PKI certs?
Does the terminal ever materialize tokens?
Where are tokens cached?
How are they renewed/revoked?
What audit information is retained?
```

These are different security decisions.

---

# 118. Implementation Phases

A staged implementation is strongly recommended.

## Phase 0 — Architecture and proof work

Deliver:

- core ADRs,
- process model prototype,
- PTY/ConPTY experiments,
- terminal engine evaluation,
- renderer prototype,
- IPC prototype,
- extension sandbox prototype.

Do not start building every provider.

---

## Phase 1 — Excellent Standalone Terminal

Build:

- PTY/ConPTY,
- terminal engine,
- GPU renderer,
- input system,
- Unicode,
- font handling,
- scrollback,
- search,
- clipboard,
- keybindings,
- configuration,
- shell launch,
- basic session management,
- cross-platform packaging.

Release criterion:

> The application is already a terminal users would choose even without extensions.

---

## Phase 2 — Session Architecture

Build:

- termd,
- session-host,
- stable IPC,
- frontend/session separation,
- crash recovery,
- reattachment,
- session metadata,
- optional detach/reattach.

Release criterion:

> Frontend restart does not unnecessarily destroy healthy sessions.

---

## Phase 3 — Command and Text Surface Platform

Build:

- command registry,
- command mode,
- keyboard fuzzy selector,
- input text surfaces,
- confirmation text surfaces,
- progress text surfaces,
- notifications,
- keybinding-to-command mapping.

Release criterion:

> Most platform functionality can be invoked through commands and terminal text surfaces without adding graphical interaction code.

---

## Phase 4 — Extension Runtime

Build:

- Wasmtime,
- WASI policy,
- component API,
- extension manifest,
- lazy activation,
- extension-host,
- permissions,
- resource limits,
- safe mode,
- SDK,
- local development tooling.

Release criterion:

> A buggy extension cannot freeze or crash normal terminal operation.

---

## Phase 5 — Resource Platform

Build:

- Resource model,
- Resource Index,
- actions,
- resource search,
- provider registration,
- context integration.

Release criterion:

> An example extension can add a new resource type without core modifications.

---

## Phase 6 — AccessPlan Engine

Build:

- typed access steps,
- progress,
- cancellation,
- errors,
- retries,
- context output,
- policy checks.

Release criterion:

> Multi-step connections can be described declaratively and executed consistently.

---

## Phase 7 — Credential Broker

Build:

- CredentialHandle,
- provider interface,
- local secure store abstraction,
- SSH agent integration,
- authentication sessions,
- expiration,
- revocation,
- secret-safe logging,
- temporary materialization.

Release criterion:

> Extensions can authenticate processes without normally receiving raw credentials.

---

## Phase 8 — DevOps Extension v1

Start narrow.

Recommended v1:

- local SSH resource discovery,
- OpenSSH orchestration,
- SSH agent support,
- known_hosts policy,
- jump hosts,
- port forwarding,
- kubeconfig discovery,
- Kubernetes context switching,
- AWS profile discovery,
- AWS CLI SSO orchestration.

Release criterion:

> Keyboard-first connection to common SSH/Kubernetes/AWS environments is materially faster than manual workflows.

---

## Phase 9 — DevOps Extension v2

Add:

- Azure,
- GCP,
- Docker,
- Teleport,
- richer Kubernetes actions,
- cloud resource discovery,
- environment-aware session contexts.

---

## Phase 10 — Credential/Vault Ecosystem

Add:

- 1Password integration,
- Bitwarden integration,
- KeePassXC integration,
- OpenBao,
- HashiCorp Vault,
- enterprise providers.

Prefer agent/temporary-credential paths before direct secret retrieval.

---

## Phase 11 — Signed Extensions

Build:

- package signing,
- publisher identity,
- permission update consent,
- revocation,
- atomic updates,
- enterprise policy.

---

## Phase 12 — Marketplace

Only after the platform is mature.

Add:

- discovery,
- publishing,
- signing enforcement,
- review/scanning,
- permissions display,
- update controls,
- enterprise private registries if needed.

---

# 119. Testing Strategy

Testing must be deep because the product combines terminal emulation, security, networking, and plugins.

## Terminal engine
- VT conformance,
- parser fuzzing,
- Unicode fuzzing,
- OSC/DCS/APC tests,
- wrapping/reflow,
- resize behavior,
- alternate screen,
- clipboard protocol.

## PTY/ConPTY
- real shells,
- interactive programs,
- signal behavior,
- resize,
- process exit,
- broken pipes,
- Windows ConPTY stress.

## Renderer
- cell rendering,
- font fallback,
- grapheme rendering,
- cursor,
- selection,
- scroll damage,
- resize,
- high-output scenarios.

## Extensions
- crash,
- infinite loop,
- memory exhaustion,
- forbidden network,
- forbidden filesystem,
- invalid manifest,
- API incompatibility.

## Credentials
- locked provider,
- expired session,
- canceled MFA,
- provider unavailable,
- credential renewal,
- revocation,
- agent unavailable.

## SSH
- unknown host,
- changed host key,
- agent signing,
- jump host,
- tunnel,
- agent forwarding.

## Cloud
Use controlled real test accounts eventually.

Test:
- AWS SSO,
- Azure browser auth,
- gcloud auth,
- session expiry,
- role/project switching.

## Teleport
Use controlled real proxy and MFA flow.

## Native platform fixtures
Exercise:
- macOS native release,
- Linux native release,
- Windows native release,
- relevant GPU backends,
- real PTY/ConPTY environments.

---

# 120. Security Testing

Continuous fuzzing should include:

- VT parser,
- Unicode,
- escape sequences,
- graphics protocol,
- shell integration,
- manifest parser,
- IPC parser,
- resource metadata parser,
- AccessPlan parser,
- provider response parsing.

Penetration testing should target:

- extension sandbox escape,
- credential leakage,
- update tampering,
- package signature bypass,
- process broker injection,
- filesystem capability bypass,
- network policy bypass,
- terminal escape abuse.

---

# 121. Failure Testing

Intentionally test:

```text
kill extension-host during action
kill frontend during SSH
kill session-host
provider returns malformed data
password manager locks mid-operation
AWS token expires
OpenBao disappears
network changes
VPN disconnects
host key changes
extension update crashes
disk becomes full
state DB corrupts
renderer device resets
```

The product should fail locally and recover predictably.

---

# 122. Release Gates

A release should not be called “production ready” solely because source code exists.

Recommended gates:

```text
source complete
      ↓
unit tested
      ↓
integration tested
      ↓
native platform tested
      ↓
controlled external provider tested
      ↓
security reviewed
      ↓
failure tested
      ↓
performance benchmarked
      ↓
release activated
```

This avoids the ambiguity of phrases such as “source-complete.”

---

# 123. Product Activation vs Implementation

Use clear status language.

Possible statuses:

```text
Proposed
Accepted
Implemented
Feature-flagged
Native-tested
Provider-tested
Security-reviewed
Beta
Generally Available
```

Do not conflate:

```text
implemented
```

with:

```text
production-ready
```

---

# 124. Performance CI

Create benchmarks for:

```text
startup
keypress latency
VT parsing
scrolling
scrollback memory
resource search
extension activation
IPC throughput
renderer frame time
session reattach
```

Track regression over time.

A new feature that slows startup should fail performance review.

---

# 125. Compatibility Strategy

The terminal should aim for:

```text
xterm compatibility
+
widely adopted modern terminal protocols
+
careful compatibility testing
```

Do not invent incompatible terminal semantics unnecessarily.

Application-specific innovations should live in:

- command layer,
- text surfaces,
- resource platform,
- extension APIs.

Keep terminal protocol behavior boring and compatible.

---

# 126. Cross-Platform Strategy

Design from the start for:

- macOS,
- Linux,
- Windows.

Do not write a Linux-only core and “port later.”

Platform abstraction should cover:

```text
PTY
windowing
global shortcuts
clipboard
secure storage
filesystem paths
process spawning
signals/job behavior
GPU backend
notifications
native packaging
updater
```

---

# 127. Privacy

The terminal handles extremely sensitive information.

Telemetry, if any, must be conservative.

Do not collect by default:

- terminal contents,
- command text,
- hostnames,
- usernames,
- resource names,
- cloud account IDs,
- vault names,
- file paths,
- environment variables.

Useful telemetry can focus on:

```text
crash type
startup timing
renderer timing
feature enablement counts
extension crash counts
anonymous performance aggregates
```

with strong opt-out/enterprise controls.

---

# 128. Local-First Design

Core functionality should remain usable offline:

- terminal sessions,
- local shell,
- command mode,
- resource cache,
- installed extensions,
- SSH to reachable networks,
- local secure storage.

Cloud resource refresh can fail independently without breaking the terminal.

---

# 129. Extension Discovery and Background Work

Extensions can maintain local caches.

Example:

```text
AWS provider
  ↓
background refresh
  ↓
resource cache
```

The user search path reads the cache.

A provider may expose:

```text
refresh()
status()
last_updated()
```

Do not let silent provider loops consume battery/network excessively.

---

# 130. Provider Health

The terminal can expose provider status:

```text
1Password   available
Bitwarden   locked
AWS SSO     expired
Teleport    authenticated, 31m remaining
OpenBao     unreachable
Kubernetes  8 contexts cached
```

This should be accessible through command mode or `terminal doctor`.

---

# 131. Authentication Text Interaction

Authentication interaction should be handled through trusted core keyboard-controlled text surfaces.

Example:

```text
Authentication required

Provider: OpenBao
Identity: engineering@example.com
Target: production SSH

Press Enter to authenticate · Esc to cancel
```

For browser/device flows, the terminal can display:

- provider,
- target,
- reason,
- status,
- cancel.

Sensitive prompts should never be arbitrary extension-rendered phishing dialogs.

---

# 132. Permission Text Interaction

Permission prompts should state:

- extension name,
- publisher,
- requested capability,
- scope,
- reason if supplied.

Example:

```text
Database Tools wants to:

Read resource metadata for:
  staging databases

Press A to allow · D to deny
```

For sensitive capabilities:

```text
AI Assistant wants to:

Read the output of the current terminal command.

1 = allow once · S = allow for session · D = deny
```

---

# 133. User-Controlled Trust

The user should be able to inspect permissions from the shell or command mode:

```text
terminal permissions
```

and see:

- installed extensions,
- granted permissions,
- provider access,
- network scopes,
- filesystem scopes,
- raw terminal access.

Permissions must be revocable.

---

# 134. Enterprise Policy

The architecture should allow later enterprise controls:

- disable third-party extensions,
- allow approved publishers,
- lock update channels,
- enforce safe-mode availability,
- forbid raw secret materialization,
- restrict network destinations,
- require specific credential providers,
- disable local password storage,
- require short-lived SSH certificates,
- enforce audit events.

Do not hard-code consumer assumptions that make enterprise policy impossible later.

---

# 135. What Not to Build First

Avoid early scope explosion.

Do not simultaneously implement:

```text
AWS
Azure
GCP
Kubernetes
Teleport
1Password
Bitwarden
KeePassXC
OpenBao
Vault
Mosh
SFTP
serial
WSL
Docker
databases
AI
marketplace
```

before the terminal core and extension APIs are stable.

Build the abstractions first.

Then add providers incrementally.

---

# 136. Anti-Patterns to Avoid

## Anti-pattern 1: Giant monolith

```text
terminal binary
├── renderer
├── AWS
├── SSH
├── Kubernetes
├── OpenBao
├── AI
└── plugins
```

This becomes unmaintainable.

---

## Anti-pattern 2: Plugins inside renderer process

A plugin bug should never freeze input or GPU rendering.

---

## Anti-pattern 3: Raw credentials everywhere

Extensions should not receive passwords/tokens unless absolutely necessary.

---

## Anti-pattern 4: Custom authentication for every provider

Use official CLIs/protocols/agents first.

---

## Anti-pattern 5: Replacing the shell

The user's shell is an ecosystem. Preserve it.

---

## Anti-pattern 6: Startup activation of all extensions

Lazy activation is essential.

---

## Anti-pattern 7: Extension access to all keystrokes

Default-deny raw input observation.

---

## Anti-pattern 8: Treating DevOps as the core product architecture

DevOps is a first-party extension/domain built on the same primitives other domains use.

---

# 137. Recommended Initial MVP

A realistic high-quality MVP could be:

### Core terminal
- Linux/macOS first if necessary, but Windows planned from architecture start,
- PTY,
- high-performance rendering,
- Unicode,
- scrollback,
- command mode,
- keybindings,
- config,
- tabs/sessions.

### Platform
- command registry,
- resource registry,
- terminal text surfaces,
- extension-host prototype,
- signed first-party extension packaging.

### DevOps v0
- parse `~/.ssh/config`,
- discover SSH hosts,
- universal fuzzy search,
- OpenSSH connection,
- ssh-agent support,
- known_hosts behavior,
- jump hosts.

User experience:

```text
shortcut
   ↓
terminal
   ↓
Ctrl+P
   ↓
type server name
   ↓
Enter
   ↓
connected
```

This is narrow but useful and validates the architecture.

---

# 138. MVP+1

Add:

- session persistence,
- kubeconfig discovery,
- Kubernetes context actions,
- AWS profile discovery,
- AWS SSO orchestration,
- richer session context.

---

# 139. MVP+2

Add:

- extension SDK,
- third-party sandboxed extension loading,
- database example extension,
- AI example extension,
- Bitwarden/1Password agent awareness,
- provider diagnostics.

This proves the platform is more than DevOps.

---

# 140. Product Positioning

Avoid describing the product only as:

```text
Ghostty-class terminal performance + Termius-class access capability, without a Termius-style GUI
```

That understates the architecture.

A stronger positioning concept is:

> **A keyboard-first extensible terminal platform that turns environments, servers, clouds, clusters, databases, and developer workflows into instantly searchable and executable resources—while delegating credentials to the security systems users already trust.**

Shorter:

> **A programmable access terminal for developers and operators.**

Or:

> **A terminal platform where every environment is one shortcut away.**

---

# 141. Core Value Proposition

The core terminal competes on:

- speed,
- compatibility,
- native feel,
- reliability,
- keyboard ergonomics.

The platform competes on:

- extensibility,
- secure capabilities,
- automation,
- resource normalization,
- context.

The DevOps extension competes on:

- fast access,
- identity orchestration,
- vault integration,
- cloud context,
- environment discovery,
- reduced command/config friction.

This separation is strategically useful.

---

# 142. The Central Abstractions

If only a few abstractions are remembered, they should be these:

```text
Command
Resource
Action
Session
SessionContext
AccessPlan
CredentialHandle
CredentialProvider
ProcessBroker
Extension Capability
```

These abstractions prevent provider-specific logic from leaking into the core.

---

# 143. Final Architecture

```text
┌──────────────────────────────────────────────────────────────┐
│                  RUST TERMINAL FRONTEND                     │
│                                                              │
│ GPU Renderer                                                 │
│ Keyboard                                                     │
│ Command Mode                                              │
│ Declarative Text Surfaces                                         │
└───────────────────────────┬──────────────────────────────────┘
                            │
                            │ IPC
                            ▼
┌──────────────────────────────────────────────────────────────┐
│                      TRUSTED CORE                            │
│                                                              │
│ Command Registry                                             │
│ Resource Registry                                            │
│ Session Supervisor                                           │
│ Extension Manager                                            │
│ Security Policy                                              │
│ Config / Storage                                             │
│ Credential Broker                                            │
│ Process Broker                                               │
│ AccessPlan Engine                                            │
└───────┬──────────────────┬──────────────────┬────────────────┘
        │                  │                  │
        ▼                  ▼                  ▼
┌───────────────┐   ┌───────────────┐  ┌────────────────────┐
│ Session Host  │   │ Extension Host│  │ Credential Services│
│               │   │               │  │                    │
│ PTY / ConPTY  │   │ Wasmtime      │  │ Agents             │
│ VT Engine     │   │ WASI          │  │ OS Keychains       │
│ Scrollback    │   │ Extensions    │  │ Vault Providers    │
│ Shell         │   │ Permissions   │  │ SSO Sessions       │
└──────┬────────┘   └───────┬───────┘  └─────────┬──────────┘
       │                    │                    │
       ▼                    ▼                    ▼
 bash/zsh/etc.       ┌─────────────┐      ┌──────────────┐
                     │ Extensions  │      │ Providers    │
                     │             │      │              │
                     │ DevOps      │      │ 1Password    │
                     │ Database    │      │ Bitwarden    │
                     │ AI          │      │ KeePassXC    │
                     │ Git         │      │ OpenBao      │
                     │ Security    │      │ Vault        │
                     └──────┬──────┘      │ OS Keychain  │
                            │             │ Cloud SSO    │
                            ▼             └──────────────┘
                ┌─────────────────────────┐
                │       Resources         │
                │                         │
                │ SSH Hosts               │
                │ AWS Accounts/Roles      │
                │ Azure Subscriptions     │
                │ GCP Projects            │
                │ Kubernetes Clusters     │
                │ Containers              │
                │ Databases               │
                │ Teleport Nodes          │
                │ Future Resource Types   │
                └───────────┬─────────────┘
                            │
                            ▼
                       Access Plans
                            │
                            ▼
                    Auth → Connect
                            │
                            ▼
                       Terminal
```

---

# 144. Final Recommendation

Build the product around this hierarchy:

```text
Terminal Engine
      ↓
Trusted Platform Core
      ↓
Capability/Security Brokers
      ↓
Extension Runtime
      ↓
Generic Resource + Action + AccessPlan Model
      ↓
First-Party Extensions
      ↓
Third-Party Ecosystem
```

Do not build:

```text
terminal
+
a giant pile of provider-specific code
```

The DevOps feature set should become a first-party extension that uses the same platform exposed to future database, Git, AI, security, and workflow extensions.

The credential system should be a broker, not a new mandatory vault.

OpenBao should be one provider, not the foundation of the whole product.

Cloud authentication should prefer temporary sessions.

SSH should use OpenSSH and agents where possible.

Extensions should use WASM/capability isolation and never sit in the render or raw-input hot path.

The terminal should remain fast and useful even if the entire extension ecosystem is switched off.

If these boundaries are preserved from the beginning, the product can grow substantially without turning into an unstable monolith.

---

# 145. Architecture Checklist

Before accepting a new core feature, ask:

- Does this introduce a graphical management screen, clickable button, host card, or mouse-only workflow? If yes, redesign it as a command/keybinding/text surface.

- Does this need to be in the startup path?
- Does this need to be in the render/input hot path?
- Is this a security enforcement boundary?
- Is this needed by essentially every extension?
- Could it be implemented safely as an extension?
- Does adding it to core create provider-specific coupling?

Before granting an extension capability, ask:

- Does it need raw terminal input?
- Does it need terminal output, and how much?
- Does it need raw credentials?
- Can a signing/opaque-handle approach work instead?
- Does it need unrestricted network access?
- Does it need unrestricted filesystem access?
- Can the scope be restricted to specific resources?

Before adding a credential provider, ask:

- Can an existing agent protocol be reused?
- Can the provider issue temporary credentials?
- Can raw secrets remain outside the terminal?
- How are sessions refreshed?
- How are credentials revoked?
- How are failures surfaced?
- What gets logged?
- What is cached locally?

Before calling a feature production-ready, verify:

- source implementation,
- unit tests,
- integration tests,
- native OS fixtures,
- controlled provider tests,
- real MFA/browser path if applicable,
- security review,
- failure tests,
- performance regression tests,
- update/migration behavior.

---

# 146. Suggested Next Engineering Documents

This blueprint should be followed by separate implementation documents:

1. **Core Terminal Architecture**
2. **Process and IPC Specification**
3. **Terminal Engine/VT Compatibility Specification**
4. **Renderer Design**
5. **Extension Runtime and WIT API**
6. **Extension Permission Model**
7. **Resource/Action Model**
8. **AccessPlan Specification**
9. **Credential Broker Security Design**
10. **DevOps Extension Design**
11. **SSH Integration Specification**
12. **Cloud Authentication Specification**
13. **OpenBao Provider ADR**
14. **Extension Signing and Marketplace Security**
15. **Testing and Native Release Matrix**
16. **Performance Budgets and Benchmark Plan**
17. **Threat Model**
18. **Data Storage and Privacy Model**

Those documents can later become the actual implementation contracts for individual teams/modules.

---

# 147. Closing Principle

The strongest version of this product is not the terminal with the largest number of built-in features.

It is the terminal with:

- an exceptionally fast and reliable core,
- very strong security boundaries,
- powerful universal abstractions,
- a keyboard-only first-party interaction model with no clickable buttons or graphical management dashboard,
- and an extension model strong enough that major product areas do not need to be baked into the core.

That architecture gives the product room to become much larger than its initial DevOps use case without sacrificing the qualities that make a terminal worth using in the first place.
