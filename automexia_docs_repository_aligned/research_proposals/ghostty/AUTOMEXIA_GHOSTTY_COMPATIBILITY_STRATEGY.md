
**Repository-alignment note.**

**State:** Strongly recommended strategy, but this file is a research proposal, not a project ADR.

The current repository already contains substantial Ghostty work. The goal is therefore to **reclassify and contain** it, not discard it.

Current source audit:
- G1–G4 substantially implemented;
- G5 partial evidence;
- top-level G6 parked-tab history accepted/implemented under real ADR 0028;
- broader topology history future.

"Core should not know Ghostty exists" means **no Ghostty authority or required dependency in core**. The profile/composition boundary may legitimately know that a Ghostty compatibility profile is selected.

Use profile revisions for safety/correctness errata rather than silently changing a released pinned profile.

---

# Automexia — Modern Terminal Compatibility and Ghostty Migration Strategy

**Document type:** Architecture / product strategy
**Status:** Recommended research strategy / integration direction — not a project authority
**Scope:** Ghostty compatibility, modern terminal compatibility, migration profiles, config import, action mapping, session-history boundaries, performance, security, testing, and product identity

---

## 1. Executive Decision

Automexia should **not** define "full Ghostty compatibility" as a primary product or architecture goal.

The correct goal is:

> **Build a modern, high-performance terminal that correctly supports the same terminal applications and protocols that work in Ghostty, while offering optional Ghostty migration profiles and compatibility tooling for users moving from Ghostty.**

Ghostty should be used as:

```text
reference implementation
+
pinned differential reference
+
migration source
```

but **not** as:

```text
the specification that defines Automexia
```

Automexia remains its own terminal platform.

The resulting priority order is:

```text
1. Automexia terminal quality
2. Modern terminal protocol compatibility
3. Automexia-native architecture and features
4. Optional Ghostty migration compatibility
```

Not:

```text
1. Match Ghostty
2. Copy Ghostty product behavior
3. Build Automexia afterward
```

---

## 2. Why Full Ghostty Product Compatibility Is Not Required

There are three very different things people may mean by "Ghostty compatibility."

### 2.1 Level 1 — Modern Terminal Compatibility

This is essential.

Automexia must correctly support modern terminal applications and protocols.

Examples:

```text
VT / xterm semantics
Unicode
grapheme clusters
wide characters
combining characters
true color
mouse reporting
bracketed paste
focus reporting
OSC hyperlinks
OSC clipboard policy
Kitty keyboard protocol
Kitty graphics where supported
alternate screen
cursor modes
terminal notifications
shell integration
```

Applications such as:

```text
vim
neovim
tmux
htop
btop
fzf
less
interactive shells
remote TUIs
```

should behave correctly.

This is not really "Ghostty compatibility."

It is **modern terminal compatibility**.

Ghostty is one useful reference implementation for verifying those semantics.

---

### 2.2 Level 2 — Ghostty Interaction Compatibility

This is optional but valuable.

Examples:

```text
Ghostty-like keybindings
Ghostty action semantics
versioned Ghostty profiles
Ghostty config import
familiar navigation behavior
```

This makes migration easier for Ghostty users.

It should be implemented as an optional compatibility layer:

```text
Automexia
    │
    ├── automexia profile
    └── ghostty-1.3 profile
```

The Automexia profile remains the default.

---

### 2.3 Level 3 — Ghostty Product Compatibility

This should **not** be a project requirement.

Examples:

```text
copying every Ghostty action
copying every config behavior
copying tab-history semantics
copying undo-close behavior
copying every lifecycle decision
copying every inspector/tool
copying window behavior
```

Automexia should implement these only if they are independently valuable to Automexia.

Ghostty behavior can inform expected UX.

It must not dictate Automexia architecture.

---

## 3. Rename the Initiative

Replace:

```text
Ghostty Compatibility — G0–G6
```

with two separate workstreams.

### 3.1 TC — Modern Terminal Compatibility

Suggested roadmap:

```text
TC0  Terminal semantics baseline
TC1  Input/keyboard protocol compatibility
TC2  Mouse/focus/paste compatibility
TC3  OSC / hyperlinks / clipboard / notifications
TC4  Graphics and advanced terminal protocols
TC5  Application compatibility and differential fixtures
```

This is core terminal work.

---

### 3.2 GM — Ghostty Migration Compatibility

Suggested roadmap:

```text
GM0  Ghostty reference fixtures
GM1  Ghostty keybinding/action profile
GM2  Ghostty config import/migration
GM3  Compatibility inspector and migration diagnostics
```

This is optional migration functionality.

---

### 3.3 Persistent Sessions Become a Separate Workstream

Things such as:

```text
parked PTYs
undo closed tabs
redo
topology history
session restoration
```

move to:

```text
Persistent Session Supervisor
```

and are governed by their own architecture and ADR.

They are **not authorized merely because Ghostty has similar functionality**.

---

## 4. Automexia Must Remain the Source of Product Identity

The default user experience must remain:

```text
profile = automexia
```

Ghostty profiles are opt-in:

```text
profile = ghostty
profile = ghostty-1.3
```

The moving alias:

```text
ghostty
```

may point to the latest supported Ghostty compatibility profile.

Pinned profiles:

```text
ghostty-1.3
ghostty-1.4
ghostty-2.0
```

must retain stable semantics.

A user choosing:

```text
ghostty-1.3
```

should not wake up after an Automexia upgrade with silently changed shortcuts.

---

## 5. Core Architecture Boundary

Ghostty compatibility must remain an adapter.

Correct dependency direction:

```text
              AUTOMEXIA CORE

CommandId
Typed Action
SessionId
PaneId
RouteId
WorkspaceId
GenerationId
TerminalEngine
Renderer
Policy
Session Supervisor
       ▲
       │
       │ typed compatibility mapping
       │
Ghostty Compatibility Adapter
```

Incorrect:

```text
Automexia Core
   ↓
Ghostty-specific actions
Ghostty-specific IDs
Ghostty-specific config types
Ghostty lifecycle assumptions
```

The core should not know that Ghostty exists.

Ghostty-specific code belongs under a namespace such as:

```text
compatibility/
└── ghostty/
```

---

## 6. Typed Action Model

Ghostty actions map onto Automexia actions.

Internally prefer:

```rust
pub enum Action {
    NewTab,
    CloseSurface,
    Scroll(ScrollAction),
    Clear(ClearAction),
    SetFontSize(FontSize),
    ToggleZoom,
    ExtendSelection(SelectionAction),
}
```

Not:

```rust
GhosttyNewTab
GhosttyScroll
GhosttyClearScreen
```

Ghostty compatibility translates external intent:

```text
Ghostty action
      ↓
Ghostty adapter
      ↓
Automexia Action / CommandId
      ↓
Policy / owner
      ↓
actual operation
```

The compatibility layer asks:

> **What did the user intend?**

It does not independently perform privileged operations.

---

## 7. Authority Boundary

The Ghostty compatibility module must never directly own:

```text
PTY lifecycle
process creation
filesystem writes
clipboard authority
credential authority
renderer device access
network access
session restoration authority
```

Correct:

```text
Ghostty profile
    ↓
Compatibility Engine
    ↓
CommandId
    ↓
Automexia Policy/Owner
    ↓
operation
```

Wrong:

```text
Ghostty Compatibility Engine
    ↓
direct PTY call
direct file write
direct clipboard
direct process launch
```

This protects Automexia's architecture even if Ghostty compatibility expands.

---

## 8. GM0 — Ghostty Reference Fixtures

Keep the fixture/provenance work already implemented.

The purpose is to establish a reproducible behavioral reference.

Example:

```text
Ghostty 1.3.1 source
        ↓
verified maintainer build
        ↓
generated actions/bindings
        ↓
versioned fixture
        ↓
Automexia compatibility tests
```

Fixtures should contain data such as:

```text
default keybindings
action names
parameter schemas
supported semantics
platform-specific differences
source version
generator version
source hash
fixture hash
```

Fixtures are **test/reference artifacts**.

They do not belong in the terminal hot path.

---

## 9. Fixture Versioning

Every fixture must be tied to:

```text
Ghostty semantic version
Ghostty source hash
generator version
platform
build configuration
```

Example:

```json
{
  "ghostty_version": "1.3.1",
  "source_sha256": "...",
  "platform": "linux",
  "generator_version": 1,
  "fixture_sha256": "..."
}
```

Never use:

```text
latest Ghostty
```

as an unversioned test dependency.

---

## 10. Native Fixture Evidence

Do not claim:

```text
macOS Ghostty compatibility
```

because Linux-generated fixtures exist.

Native release evidence remains platform-specific.

Required evidence may include:

```text
Windows
Linux
macOS
```

where Ghostty semantics differ.

If a native generator cannot be run:

```text
status = external/native pending
```

rather than synthetic pass.

---

## 11. Ghostty as Differential Reference

Ghostty can be useful beyond shortcut fixtures.

Automexia can compare terminal behavior against Ghostty for carefully chosen deterministic scenarios.

Example:

```text
input escape sequence
      ↓
Ghostty terminal state
Automexia terminal state
      ↓
compare expected semantics
```

Good differential targets:

```text
clear behavior
cursor modes
alternate screen
scroll regions
selection behavior
keyboard protocol output
mouse protocol output
OSC behavior
Unicode edge cases
```

Do not blindly declare Ghostty correct for every possible behavior.

Prefer:

```text
standards
+
upstream protocol documentation
+
Ghostty reference
+
Automexia compatibility tests
```

in that order.

---

## 12. GM1 — Keybinding Engine

Keep the typed keybinding engine.

It improves Automexia independently from Ghostty.

Recommended architecture:

```text
config/profile change
      ↓
parse
      ↓
validate
      ↓
compile immutable binding registry
      ↓
atomic publish
      ↓
keypress lookup
```

The keystroke path must remain:

```text
allocation-free or effectively allocation-free
no filesystem
no network
no IPC
no config parsing
no profile compilation
no Ghostty process
```

---

## 13. Binding Data Model

A binding should separate:

```text
trigger
conditions
action
origin
priority
profile
```

Example:

```rust
pub struct Binding {
    pub trigger: Trigger,
    pub action: Action,
    pub origin: BindingOrigin,
    pub condition: Option<BindingCondition>,
}
```

Origins may include:

```text
AutomexiaDefault
GhosttyProfile
UserConfig
WorkspaceConfig
ExtensionContribution
```

This helps:

```text
inspection
conflict resolution
migration
debugging
palette hints
```

---

## 14. Binding Lookup Performance

Target:

```text
keypress
   ↓
indexed lookup
```

not:

```text
keypress
   ↓
linear scan of all bindings
```

Use immutable indexes for:

```text
logical key
physical key
named key
modifier combinations
sequence/chord roots
```

Performance gates should include:

```text
single binding lookup
100 bindings
1,000 bindings
10,000 bindings
sequence start
sequence continuation
invalid sequence replay
```

The compatibility feature must introduce no measurable typing-latency regression.

---

## 15. Chords and Sequences

Multi-key sequences must preserve terminal input.

Example:

```text
Ctrl+X
   ↓
pending sequence
   ↓
unexpected key
```

The exact original input must be replayed when the sequence does not resolve.

Automexia must not swallow terminal input.

Required tests:

```text
valid sequence
invalid second key
timeout
profile reload while pending
pane switch
session close
IME
terminal application owning similar keys
```

---

## 16. Transactional Reload

Profile/config reload must be transactional.

Correct:

```text
parse new config
    ↓
validate
    ↓
compile new registry
    ↓
success?
   /     \
 yes      no
  │       │
publish   keep old registry
```

Never:

```text
partially mutate active bindings
then discover config error
```

---

## 17. Ghostty Profiles

Provide:

```text
automexia
ghostty
ghostty-1.3
```

The default is:

```text
automexia
```

Pinned profile behavior must be immutable.

Unknown profile:

```text
fail during config validation
```

not:

```text
silently fall back
```

---

## 18. Ghostty Action Catalog

Maintaining a full upstream action catalog is useful for diagnostics.

Each Ghostty action should be classified:

```text
Supported
Adapted
Unavailable
Deferred
Dangerous / explicitly unsupported
```

### Supported

Direct Automexia equivalent.

### Adapted

Intent supported through translation.

Example:

```text
Ghostty positive scroll = down
Automexia internal positive = up
```

Adapter flips the sign.

### Unavailable

No safe/current Automexia equivalent.

The user receives a clear diagnostic.

### Deferred

Potentially useful, but not part of current compatibility scope.

---

## 19. Unsupported Actions Must Never Fall Through to the Shell

If Ghostty config references:

```text
unsupported action
```

Automexia must not:

```text
silently ignore
send raw action text to shell
execute arbitrary command
```

Instead:

```text
binding disabled
+
clear diagnostic
+
compatibility inspector status
```

---

## 20. Action Semantics Testing

For every mapped action, test:

```text
input parameters
boundary values
platform differences
alternate screen
empty state
full state
disabled/unavailable state
```

Examples:

```text
clear screen
clear history
scroll lines
scroll page
scroll fraction
font size
selection extension
zoom
split navigation
```

Compatibility belongs in adapter-level tests.

Automexia core semantics remain independently tested.

---

## 21. Ghostty `clear_screen` Compatibility

Ghostty-specific semantics should be translated exactly in the compatibility layer.

Automexia may still retain its own richer native actions such as:

```text
clear visible
clear history
clear both
```

Ghostty profile maps its action to the closest exact supported Automexia operation.

Do not globally change Automexia semantics merely to match Ghostty.

---

## 22. Scroll Translation

If Ghostty's parameter direction differs from Automexia internal direction, translate at the adapter.

Example:

```text
Ghostty +1 row
    ↓
Compatibility Adapter
    ↓
Automexia -1 internal delta
```

Do not reverse Automexia's native scroll convention project-wide.

This principle applies to every unit/sign/origin mismatch.

---

## 23. Font Size

Ghostty absolute font-size actions should map to Automexia's typed font-size owner.

Automexia remains free to enforce safety limits such as:

```text
minimum font size
maximum font size
finite value
```

Compatibility means preserving user intent within Automexia safety constraints.

---

## 24. GM2 — Ghostty Config Import

Config migration is optional migration UX.

It is not required for terminal correctness.

Recommended command:

```text
automexia compatibility import ghostty
```

or equivalent.

Default behavior:

```text
preview only
```

Apply requires explicit user approval.

---

## 25. Migration Security

Treat Ghostty config as untrusted input.

Requirements:

```text
bounded total bytes
bounded include depth
canonicalized include paths
cycle detection
symlink/reparse protection
no shell evaluation
no arbitrary code execution
no network fetches
no writes during preview
backup before apply
atomic update
```

If an unsupported directive appears:

```text
report it
```

rather than guessing.

---

## 26. Migration Scope

Initially import only safe useful concepts:

```text
keybindings
config includes where safely resolvable
simple profile-compatible values
```

Do not try to reproduce every Ghostty config feature.

The goal is:

```text
reduce migration friction
```

not:

```text
build a full Ghostty config interpreter
```

---

## 27. Migration Report

Generate a report:

```text
Imported
Adapted
Skipped
Unsupported
Conflicting
```

Example:

```text
Imported:
✓ 42 keybindings

Adapted:
✓ scroll direction
✓ font-size command

Skipped:
○ unsupported platform-only action

Conflicts:
! Ctrl+Shift+P already bound by Automexia
```

No silent semantic loss.

---

## 28. GM3 — Compatibility Inspector

Keep the compatibility inspector.

It is useful for debugging:

```text
active profile
binding source
resolved action
adaptation
unbound state
sequence state
```

Example:

```text
Profile: ghostty-1.3
Trigger: Ctrl+Shift+T
Action: NewTab
Origin: Ghostty default
Status: Supported
```

Sensitive data must remain excluded.

Inspector must not expose:

```text
environment values
credentials
clipboard contents
hidden terminal output
shell command history
private paths unless explicitly requested
```

---

## 29. Generated References

Generated compatibility references should come from the same compiled registry used at runtime.

Do not maintain:

```text
runtime table
+
separate handwritten docs table
```

because they will drift.

Preferred:

```text
compiled registry
      ↓
runtime
      ↓
CLI inspector
      ↓
palette hints
      ↓
generated Markdown reference
```

one source of truth.

---

## 30. Ghostty Tooling Must Stay Off the Hot Path

Tools such as:

```text
fixture generator
config migration
compatibility inspector
reference generator
checksum verifier
```

must load only when invoked.

No:

```text
background Ghostty process
periodic Ghostty fixture refresh
network Ghostty lookup
```

during normal terminal operation.

---

## 31. G6 Must Move Out of Compatibility

The following are not ordinary compatibility behavior:

```text
parked PTYs
undo closed tab
redo
persistent session history
split restoration
window restoration
```

Move them into:

```text
Persistent Session Supervisor
```

with its own ADR.

Compatibility mapping may later invoke the Automexia feature.

Example:

```text
Ghostty undo-close action
       ↓
compatibility mapping
       ↓
Automexia SessionHistory::Undo
```

But Ghostty compatibility does not authorize or implement the session-lifecycle feature itself.

---

## 32. Why Parked PTYs Need Separate Governance

Closing a tab usually implies:

```text
shell/process terminates
```

Parking means:

```text
UI disappears
process remains alive
network may remain active
remote session may remain authenticated
```

This changes:

```text
RAM
process count
file descriptors
network connections
credential lifetime
remote resource use
user expectations
```

Therefore it requires:

```text
resource policy
security policy
visibility
cleanup
credential revalidation
```

---

## 33. Persistent Session Model

A parked session should be represented by stable Automexia identity.

Example:

```rust
pub struct ParkedSession {
    pub session_id: SessionId,
    pub generation: GenerationId,
    pub route: RouteId,
    pub context: SessionContext,
    pub principal: PrincipalId,
    pub parked_at: Instant,
}
```

Never restore based only on:

```text
tab position
tab number
window position
```

---

## 34. Resource Limits for Parked Sessions

The Persistent Session Supervisor must enforce limits.

Examples:

```text
max parked count
max memory
max per workspace
TTL
idle duration
process count
handle/fd count
```

When limits are exceeded:

```text
oldest/lowest-priority parked session
      ↓
graceful shutdown
      ↓
forced cleanup if needed
```

Exact limits come from profiling and product policy.

---

## 35. Parked Sessions Must Be Visible

Do not keep invisible persistent sessions indefinitely.

Provide:

```text
sessions active
sessions parked
sessions kill <id>
sessions clear-parked
```

The Connection/Session Hub should expose:

```text
Active: 4
Parked: 2
```

and contextual details.

Users should know when remote sessions remain alive.

---

## 36. Status Refresh After Restore

A parked session may outlive:

```text
AWS credential
SSH certificate
Teleport certificate
cloud session
Kubernetes token
```

On restore, Automexia resumes the same live parked topology and its exact PTYs;
it does not relaunch the shell or remote process.

Restoration must not be blocked on credential reauthentication or current token
status. Doing so would change the accepted ADR 0028 undo semantics.

After restore, context/provider adapters may refresh public status and mark it
stale or expired. Any new privileged, provider, or remote operation still needs
current authorization and, where required, fresh review/authentication.

```text
restore exact parked topology
    ↓
refresh public status asynchronously
    ↓
new privileged operation? require fresh review/auth
```

---

## 37. Performance Guardrails

Ghostty compatibility must not impact the terminal hot path materially.

Required hot-path invariants:

```text
no allocation where practical
no filesystem
no network
no subprocess
no config parse
no profile compile
no Ghostty process
bounded lookup
```

Benchmark:

```text
keypress lookup
sequence start
sequence continuation
invalid sequence replay
profile switch
1k/10k bindings
```

Add regression budgets to CI.

---

## 38. Security Guardrails

Ghostty compatibility expands parsing and migration surfaces.

Required security controls:

```text
untrusted config parsing
bounded data
no shell evaluation
typed actions
no arbitrary process authority
no direct clipboard/filesystem authority
no session restore authority
no credential access
```

Every compatibility action still goes through normal Automexia ownership/policy.

---

## 39. Product Identity Guardrails

Never make compatibility features dominate Automexia branding or default UX.

Correct:

```text
Automexia
├── native experience
├── extension platform
├── secure automation
└── optional compatibility profiles
```

Wrong:

```text
Automexia
= Ghostty-compatible terminal with extra features
```

Ghostty should be mentioned mainly in:

```text
migration
compatibility profiles
developer/testing docs
```

not as the architecture's defining identity.

---

## 40. Test Strategy

Compatibility requires multiple test layers.

### 40.1 Fixture tests

Validate generated source/reference data.

### 40.2 Unit tests

Test parsing, triggers, action mapping, collisions, adaptation.

### 40.3 Property tests

Test deterministic registry compilation and sequence behavior.

### 40.4 Fuzz tests

Targets:

```text
Ghostty binding text
config includes
malformed Unicode
include graphs
cycles
oversized config
action parameters
```

### 40.5 Differential tests

Compare selected terminal semantics against standards/upstream/Ghostty.

### 40.6 Native tests

Windows/Linux/macOS where behavior is platform-dependent.

### 40.7 Performance benchmarks

Hot-path lookup and registry compilation.

---

## 41. Fuzzing Requirements

Config/binding fuzzers must verify:

```text
no panic
bounded memory
bounded recursion/include depth
no shell
no network
no user-path writes
deterministic rejection
```

Every discovered crash becomes a permanent regression corpus item.

---

## 42. Compatibility Evidence

Use the Automexia Evidence Graph.

Example:

```text
GM1 Ghostty profile
   ↓
requirement
   ↓
fixture test
   ↓
Windows native evidence
   ↓
commit
   ↓
artifact
```

Statuses remain dimensional:

```text
source complete
contract complete
Windows native
Linux native
macOS native
performance
accessibility
release
```

Avoid vague:

```text
Ghostty compatibility complete
```

---

## 43. Version Support Policy

Automexia should not support every Ghostty release forever.

Define a compatibility window.

Example policy:

```text
latest Ghostty profile
+
one or two pinned prior major/minor families
```

Old profiles may remain usable if maintenance cost is low, but new fixture generation/testing is not unlimited.

Document:

```text
supported
maintenance
deprecated
removed
```

states.

---

## 44. Moving Alias Policy

The alias:

```text
ghostty
```

should map to the newest supported profile.

The profile:

```text
ghostty-1.3
```

remains fixed.

Changing:

```text
ghostty
```

after an Automexia update is acceptable because it is explicitly a moving alias.

Changing:

```text
ghostty-1.3
```

is not.

---

## 45. Compatibility Must Not Change Core Storage Schemas

Avoid storing:

```text
GhosttyActionId
GhosttyBinding
GhosttyProfileInternal
```

inside core persisted structures.

Persist generic Automexia structures plus compatibility metadata where necessary.

This makes compatibility removable without migration damage.

---

## 46. Compatibility Must Not Change Session Identity

Never replace Automexia:

```text
SessionId
PaneId
RouteId
WorkspaceId
GenerationId
```

with Ghostty-style identity.

Ghostty only specifies requested action semantics.

Automexia determines:

```text
which session
which resource
which generation
which authority
```

---

## 47. Compatibility Must Not Control Renderer Internals

Ghostty action semantics can request:

```text
font size
zoom
scroll
selection
```

through typed Automexia actions.

The compatibility layer does not get:

```text
GPU device
render pipeline
font cache internals
PTY grid internals
```

This preserves renderer independence.

---

## 48. Recommended Repository Structure

```text
crates/
├── terminal-core/
├── commands/
├── sessions/
├── renderer/
├── policy/
├── config/
│
└── compatibility/
    └── ghostty/
        ├── types/
        ├── parser/
        ├── registry/
        ├── profiles/
        ├── action-map/
        ├── migration/
        ├── inspector/
        ├── fixtures/
        └── tooling/
```

Or equivalent workspace crates.

The key rule is dependency direction:

```text
compatibility → Automexia APIs
```

never:

```text
Automexia APIs → compatibility
```

---

## 49. Recommended Roadmap

### Phase A — Terminal Correctness

Prioritize:

```text
VT/xterm
Unicode
keyboard protocols
mouse
OSC
graphics
application compatibility
```

before migration convenience.

### Phase B — Keep Useful Existing G0–G5 Work

Retain:

```text
fixtures
typed keybinding engine
action registry
profiles
config migration
inspector
fuzzing
benchmarks
generated references
```

but reclassify them under GM.

### Phase C — Automexia-Native Features

Prioritize:

```text
extension platform
secure access
resource/context model
Automexia Lab
policy
persistent session supervisor
```

according to the main architecture roadmap.

### Phase D — Persistent Sessions

Implement only after its dedicated ADR and resource/security model.

### Phase E — Additional Compatibility

Only add new Ghostty parity when:

```text
real users request it
+
maintenance cost is justified
```

---

## 50. What to Stop Doing

Do not continue expanding the project with the goal:

```text
make every Ghostty feature work in Automexia
```

Do not automatically implement:

```text
new Ghostty action
new Ghostty config field
new Ghostty lifecycle behavior
```

simply because upstream adds it.

Every addition must answer:

```text
Does this improve Automexia users?
Does it make migration materially easier?
Does it preserve architecture?
Is the maintenance cost justified?
```

If not:

```text
mark unavailable/deferred
```

and move on.

---

## 51. What to Keep from the Existing Agent Work

Keep:

```text
G0 fixture/provenance infrastructure
G1 typed binding engine
G2 versioned profiles
G2 safe migration
G3 sequence/dispatch infrastructure
G4 action adaptation
G5 inspector
G5 generated reference tooling
G5 fuzzing
G5 benchmarks
```

These improve the platform independently.

Rename/reclassify them.

Do not throw away sound engineering just because the original scope was too broad.

---

## 52. What to Move Out

Move:

```text
parked PTYs
closed-tab history
undo/redo session lifecycle
split/window restoration
```

to:

```text
Persistent Session Supervisor
```

These require separate:

```text
ADR
resource limits
credential policy
visibility
native lifecycle testing
```

---

## 53. Potential ADR Topics and Integration

Treat these as research topics, not instructions to create parallel decisions.
Accepted ADR 0028 already owns bounded complete top-level tab parking; broader
topology or changed authority requires an explicit extension or superseding ADR.

```text
Modern Terminal Compatibility Scope
Ghostty Migration Compatibility Scope
Versioned Compatibility Profiles
Ghostty Fixture Provenance
Config Migration Security
Persistent Session Supervisor
Parked Session Resource Policy
Session Restore Identity and Credential Revalidation
```

State explicitly:

> **Ghostty compatibility does not authorize process/session-lifecycle semantics outside the compatibility adapter.**

---

## 54. Acceptance Criteria for the Compatibility Layer

A Ghostty compatibility release is acceptable when:

```text
[ ] Automexia remains default profile
[ ] Ghostty profile is opt-in
[ ] pinned profiles are immutable
[ ] fixture provenance verified
[ ] typed action mapping only
[ ] unsupported actions are explicit
[ ] no direct process authority
[ ] no direct PTY authority
[ ] no direct credential authority
[ ] migration is preview-first
[ ] migration is bounded/fuzzed
[ ] hot-path lookup meets performance budget
[ ] invalid chords replay exact input
[ ] action adaptation tests pass
[ ] native platform evidence is honest
[ ] generated docs match runtime registry
[ ] G6/session-history behavior is outside this release gate
```

---

## 55. Acceptance Criteria for Persistent Session History

Separately:

```text
[ ] dedicated ADR accepted
[ ] visible parked-session state
[ ] max count
[ ] TTL
[ ] memory/process/handle limits
[ ] cleanup policy
[ ] graceful/forced termination
[ ] public status refresh after restore; fresh authorization for new operations
[ ] restore uses stable IDs/generation
[ ] network sessions remain visible
[ ] shutdown behavior defined
[ ] 1/10/50-session resource tests
[ ] native lifecycle tests
```

Only after these pass should undo-close/persistent-session features activate broadly.

---

## 56. Final Target Product Identity

Automexia should be described as:

> **A fast, modern, keyboard-first terminal platform with secure automation and an extensible capability model, offering optional migration profiles for users coming from terminals such as Ghostty.**

Not:

> **A Ghostty-compatible clone with extra extensions.**

The relationship should be:

```text
Ghostty
   ↓
reference / migration source

Automexia
   ↓
independent product
```

---

## 57. Final Architecture

```text
                      AUTOMEXIA
                          │
              ┌───────────┼───────────┐
              ▼           ▼           ▼
        Terminal Core   Commands    Sessions
              │           │           │
              │           │           │
              └───────┬───┴───────┬──┘
                      │           │
                      ▼           ▼
               Automexia APIs   Policy
                      ▲
                      │
             typed compatibility map
                      │
              ┌───────┴────────┐
              ▼                ▼
       Ghostty Profile    Ghostty Migration
       / Action Adapter      / Inspector
              │
              │
        fixtures/reference
```

Separately:

```text
Persistent Session Supervisor
        │
        ├── detach
        ├── park
        ├── undo close
        └── restore
```

The two systems may integrate through typed commands, but they are not the same subsystem.

---

## 58. Final Recommendation

The optimal direction is:

```text
KEEP
    modern terminal compatibility
    Ghostty fixture/reference tests
    typed binding/action engine
    versioned Ghostty profiles
    safe config migration
    compatibility inspector
    fuzz/benchmark tooling

NARROW
    Ghostty parity as a product goal

MOVE
    G6 parked-session/history behavior
    into Persistent Session Supervisor

PRESERVE
    Automexia profile as default
    Automexia IDs
    Automexia policy
    Automexia command system
    Automexia PTY/session ownership
    Automexia renderer ownership
```

Ghostty should help Automexia become **more compatible and easier to migrate to**.

It should never become the architecture that Automexia has to follow.
