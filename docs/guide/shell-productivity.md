# Shell integration and command productivity

This page is the canonical product guide for shell integration, completion, Quick Actions, and persistent aliases. It replaces the previous split between `SHELL-INTEGRATION.md`, `COMMAND-PRODUCTIVITY.md`, `DEVOPS-ALIASES.md`, and the compatibility/threat-model pages.

## Status at a glance

| Capability | Status | Important boundary |
|---|---|---|
| Session-only PowerShell/CMD/Bash/Zsh/Fish integration | **Available now** | Normal launch does not rewrite shell profiles. |
| Native shell completion adapters | **Implemented locally; release evidence gated** | The shell editor owns the buffer, cursor, history, quoting, and menu. |
| Typed Quick Action model and private store | **Implemented locally** | Actions have no implicit execution authority. |
| Reviewed search/editor + insert/copy | **Implemented locally** | Insertion is explicit; terminal output is never auto-executed. |
| Five-shell alias projection and opt-in persistence | **Implemented locally** | Generated files are derived artifacts; activation is explicit and reversible. |
| Static first-party DevOps action packs | **Implemented locally** | Packs are disabled by default and collision-checked. |
| Selected native alias import and trusted workspace task bridges | **Implemented locally** | Import/trust is explicit, bounded, revocable, and insertion-only. |
| Provider/capsule-aware actions | **Planned** | Requires managed session/provider context. |
| App-owned inline suggestion UI | **Planned** | Native completion remains the fallback. |
| Ecosystem/AI action packs | **Deferred** | Requires later signing/sandbox/security gates. |

## Design principle

Automexia does not become another shell. PowerShell/PSReadLine, Bash/Readline, Zsh/ZLE, Fish, and CMD continue to own command editing, history, quoting, cursor semantics, and normal completion. Automexia contributes bounded metadata, generated adapters, and typed actions around that native experience. This prevents the renderer from reconstructing commands from terminal cells and keeps the security boundary auditable.

Automexia's shell integration makes prompt context, resize-safe semantic rows,
session cloning, and icon-aware listings work without changing the objects or
bytes that scripts consume.

## Session-only integration by default

`cargo dev`, `cargo automexia`, and packaged Automexia releases load
integration only into the child shell they start. A normal application launch
does not execute an installer, write a PowerShell profile, change execution
policy, start WSL for provisioning, or create persistent shell files.

Release builds resolve only a complete `shell-integration/` resource tree
adjacent to the signed executable. Debug builds additionally accept the
repository path supplied directly by `cargo automexia`. The internal
`AUTOMEXIA_SHELL_INTEGRATION_ROOT` value is replaced after configuration is
loaded; release builds do not trust an arbitrary inherited/configured path.
Missing resources degrade to the user's unmodified shell instead of causing a
profile mutation or interpreter launch.

| Host/session | Session behavior |
|---|---|
| Windows PowerShell / PowerShell 7 | The signed package script is sourced into the child session for prompt lifecycle metadata, OSC 7 directory, shell/user identity, same-pane CMD interception, and a PowerShell formatting view. |
| Command Prompt | UTF-8 setup, full-path semantic prompt, identity metadata, and `ls`/`ll` wrappers while built-in `dir` and explicit `cmd /c` remain native. |
| WSL Bash/Zsh/Fish | Existing explicit persistent integration is honored; normal launch does not rewrite a distribution. |
| Linux/macOS Bash/Zsh/Fish | Existing shell-native hooks are honored; normal launch does not rewrite profiles or terminfo. |

The integration loads only when `TERM_PROGRAM=Automexia`,
`AUTOMEXIA_SHELL_INTEGRATION=1`, or the corresponding WSL marker is present.
It uses load guards so repeated sourcing cannot stack prompt hooks.

Persistent integration for nested shells opened outside Automexia remains
available, but consent is explicit:

```text
automexia shell-integration doctor
automexia shell-integration install [--force] [--quiet]
automexia shell-integration uninstall [--quiet]
```

`doctor` is read-only. Install/uninstall uses the packaged, bounded resource
root and the platform installer. On Windows it invokes PowerShell without
`-ExecutionPolicy Bypass`; enterprise `AllSigned` or Group Policy decisions
therefore remain authoritative. Release packages Authenticode-sign and
timestamp every distributed `.ps1` and `.ps1xml` asset before MSI/ZIP
creation. The compatibility installer retains atomic/idempotent profile-block
handling, precise OneDrive Cloud Files reparse-tag acceptance, and strict
symlink/junction rejection.

For session-only Windows integration, Automexia canonicalizes and validates the
resource directory before launch, then uses a normal local drive path when that
path has an equivalent safe representation. This avoids presenting a local
development script as a verbatim/UNC-like path under RemoteSigned. Automexia
does not alter any execution-policy scope, unblock files, or evaluate script
text. If an authoritative policy still denies the script (for example,
AllSigned with an unsigned developer checkout), startup falls back quietly to
the native shell; the shell-integration doctor reports the resource boundary
and signed release packages provide the supported integrated path.

## Native command completion

Automexia provisions shell adapters but never computes candidates from rendered
terminal cells. Tab behavior, quoting, cursor movement, history, menus,
autosuggestions, and accessibility remain owned by PSReadLine, Readline, ZLE,
or Fish. Existing native definitions win on Bash, Zsh, and Fish. PowerShell
requires an explicit override because it has no supported public read-only
completer registry; CMD retains native fallback without a parity claim.

```text
cargo xtask completion doctor
cargo xtask completion refresh --provider docker --shell bash
cargo xtask completion refresh --provider kubernetes --shell zsh
cargo xtask completion refresh --provider helm --shell fish
cargo xtask completion refresh --provider kubernetes --shell powershell --allow-native-override
```

Refresh is the only operation that starts a provider. It uses the installed
official CLI with exact arguments and no stdin, a 750 ms deadline, 1 MiB stdout
and 256 KiB stderr ceilings, fixed private destinations, SHA-256 sidecars, and
atomic replacement. Provider processes receive an explicit secret-free
environment with a local absolute-only helper `PATH`; they do not inherit home,
provider configuration, proxy, credential, or arbitrary application variables.
Restart the shell after refresh, enable, disable, or remove.
`doctor` resolves executable names without executing them and validates each
bounded artifact, digest, metadata record, provenance
header, PowerShell consent marker, and managed parent chain. It does not run a
provider, authenticate, read command history, or access secrets.

Disable or remove managed state without affecting native completion:

```text
cargo xtask completion disable
cargo xtask completion enable
cargo xtask completion remove --provider docker --shell bash
```

Generated files live below
`<Automexia-config-root>/generated/completion/<shell>/`. They are disposable;
the provider remains their source. Do not edit them. A digest mismatch, linked
artifact or managed parent, non-directory path component, oversized file,
unsupported shell/provider, missing tool, timeout, or malformed output fails
closed to native shell behavior.

During refresh, a two-digest sidecar temporarily accepts the verified previous
or candidate artifact, so interruption never requires sourcing unverified text
and does not discard the last usable cache. The successful steady state contains
one digest.

`AUTOMEXIA_CONFIG_HOME`, when set, must be absolute. The native defaults are
`%LOCALAPPDATA%\Automexia\Terminal` on Windows,
`~/Library/Application Support/io.github.AmjedAllaya.AutomexiaTerminal` on
macOS, and `${XDG_CONFIG_HOME:-$HOME/.config}/automexia` on Linux/BSD. Managed
completion state enforces a 4096-byte root ceiling; Windows completion state is
local-drive-only and rejects UNC roots before any startup filesystem probe.
Windows installer and PowerShell adapter hashing use the platform SHA-256 API
directly, so clean install and shell startup do not depend on PowerShell module
auto-loading.
Managed profile blocks are updated in place when their owned source line becomes
stale; uninstall validates every exact owned target before changing profiles or
files.

CP3.1 persistent user aliases reuse the shipped CP1 managed integration block;
no second profile block or per-alias profile edit is installed. When explicitly
enabled, PowerShell, Bash, Zsh, Fish, and CMD derive their platform-private
configuration root, reject linked/reparse or permission-unsafe state, verify the
exact ordered ten-line `automexia-devops/0.4.0` content-addressed generation
manifest and shell artifact, recheck late native collisions, and load one bounded
generated file. A self-consistent manifest from any other compiler is tampered
state and cannot replace the last-known-good aliases. Fish batches only the
fixed metadata/digest probes in one constant bounded helper; generated provider
or action text is never passed to it. PowerShell/Bash/Zsh/Fish support explicit
last-known-good reload; CMD asks for a new session because safe
in-process DOSKEY ownership cannot be proven. Uninstall removes only the exact
validated generated-alias topology and preserves canonical Quick Actions. The
full typed-source, collision, recovery, and command contract is specified in
[DevOps Quick Actions and persistent aliases](shell-productivity.md).

On Windows, detected PowerShell profiles may live below a OneDrive-redirected
Documents folder. Provisioning inspects the native reparse tag of every existing
profile-path component: Microsoft Cloud Files tags are allowed, while symbolic
links, junctions, other name-surrogate redirects, and unknown reparse types fail
closed before profile directories are created or files are changed. The same
classification protects install, stamped no-op/repair, and uninstall. Automexia-
owned `%LOCALAPPDATA%` destinations retain their stricter no-reparse-point rule.

Explicit Windows-to-WSL install/uninstall sends at most 6 MiB of raw UTF-8 bytes
through redirected standard input to a fixed
`wsl.exe --distribution <allowlisted-token> --exec sh -s` process. No source
text, encoded command, decoder pipeline, or `sh -c` string appears in the
Windows command line. Standard output/error are captured per distribution, null
bytes and unsafe distribution tokens are rejected, and Docker Desktop internal
distributions remain excluded.

## Prompt ownership

The integration emits OSC 7, OSC 133, and bounded OSC 1337 user variables for
the shell, user, distribution, prompt generation, and lifecycle. The renderer
owns context/path semantic rows; PSReadLine, Readline, or ZLE owns only the
lambda, editable input, and cursor. This split is why completed prompts can be
reflowed during resize without asking a shell editor to reconstruct scrollback.

Metadata is advisory and validated. An absolute, control-free OSC 7 directory
can update a pane's launch descriptor; malformed or relative values cannot.
Context discovery is asynchronous, bounded, session-scoped, and never blocks
the PTY/render path.

## Icon-aware listings

PowerShell keeps real `DirectoryInfo` and `FileInfo` pipeline objects and
changes presentation only. The final `Name` field contains the composite icon
beside the filename; there is no separate `Icon` column. CMD delegates its
interactive `ls` view through the same safe formatter. Bash/Zsh use `eza` when
installed and fall back to the native command when it is absent.

Folder categories such as secrets, configuration, logs, source, tests, build,
assets, packaging, infrastructure, caches, and documentation use a folder
shape plus a small semantic badge. The complete category/color contract is in
[Liquid Hacker UX](terminal-experience.md#file-and-folder-icons).

Set `AUTOMEXIA_PLAIN_LS=1` before the integration loads to disable all listing
presentation. In Bash/Zsh, `command ls` bypasses the wrapper for one command.
In PowerShell, explicit object pipelines and property access remain native.
Set `AUTOMEXIA_PLAIN_CMD=1` to keep a typed `cmd`/`cmd.exe` launch completely
native instead of applying the same-pane CMD integration.

## Persistent repair and removal

Prefer the explicit application commands because they resolve the installed
resource root and preserve the host execution policy:

```text
automexia shell-integration doctor
automexia shell-integration install --force
automexia shell-integration uninstall
```

Repository scripts remain available to maintainers and isolated tests:

```powershell
powershell -NoProfile -File shell-integration/install-windows.ps1 -Force
powershell -NoProfile -File shell-integration/uninstall-windows.ps1
```

```bash
bash shell-integration/install-unix.sh --force
bash shell-integration/uninstall-unix.sh
```

Installer tests use isolated profile roots and verify first install, no-op
repeat, deliberate damage, repair, malformed markers, reparse-point rejection,
raw WSL transport, and uninstall boundaries.

## Why injection instead of a second shell

Automexia hosts the user's real PowerShell, CMD, WSL, Bash, or Zsh process. It
does not replace the shell or parse commands itself. Small guarded hooks expose
metadata the PTY protocol cannot otherwise know, preserve native history and
profiles, and can be removed independently. Session-only sourcing is the normal
boundary; persistent profile changes require an explicit maintenance command.
See [ADR 0017](../project/adr/0017-session-only-shell-integration.md), which supersedes
[ADR 0009](../project/adr/0009-launch-time-shell-provisioning.md).


## Quick Actions

A Quick Action is a typed, reviewable command template rather than an opaque shell snippet. The canonical source stores a stable action ID, title/description, tokenized command, typed placeholders, risk class, scope, tags, context requirements, and revision metadata. Generated shell aliases are projections of that source; they are disposable and must never become the authoritative copy.

The user flow is intentionally review-first:

1. Search actions from the palette or management surface.
2. Inspect the exact command tokens, placeholders, scope, risk, collision state, and tool health.
3. Fill explicit placeholders. Secret values are not persisted into action definitions or diagnostic output.
4. Choose **Insert** or **Copy**. Exact launch remains a separate capability boundary and is not silently granted by saving an action.
5. Optionally publish a collision-free alias for selected shells. Persistent changes are opt-in, transactional, reversible, and limited to Automexia-owned generated blocks/artifacts.

Scope precedence is deterministic. Session-only or capsule-aware values can override a trusted workspace action; a trusted workspace action can override an explicitly selected user action; built-in packs remain lowest-priority suggestions. Native shell definitions win unless the user explicitly accepts an Automexia-owned binding where the platform can prove ownership safely.

## Persistent aliases

Persistent aliases are compiled from canonical typed actions into platform-specific generated files for PowerShell, Bash, Zsh, Fish, and CMD. Publication is generation-based: a complete candidate is written and verified before the active pointer/manifest changes, so a crash cannot expose a half-old/half-new set. Digest, owner, compiler identity, permissions/reparse state, collision state, and the active generation are rechecked before a shell loads the artifact.

The generated aliases do not contain credentials or provider tokens. They are not synchronized automatically between machines, WSL distributions, remote hosts, or containers. Each environment has its own trust and filesystem boundary.

### Alias management CLI

The implemented management surface is:

CP3.1 exposes scriptable, noninteractive management:

```text
automexia aliases list [--shell <shell>] [--json]
automexia aliases preview [--shell <shell>] [--show-source] [--json]
automexia aliases test [--shell <shell>] [--json]
automexia aliases enable <action-id> --name <name> --shell <shell>... [policy flags]
automexia aliases disable <action-id> [--shell <shell>]
automexia aliases rename <action-id> <name>
automexia aliases regenerate
automexia aliases disable-all
automexia aliases rollback <current-generation>
automexia aliases doctor [--json]
automexia aliases reload --shell <shell>
```

`enable`, `disable`, `rename`, `regenerate`, and `disable-all` are non-mutating
previews unless `--apply` is present. Applied source changes require
`--expected-revision` and `--expected-generation`; generation-only operations
require the expected generation. `rollback` also requires explicit `--apply`.
Stable JSON/text output exposes the proposed source revision/digest, current CAS
revision and generation, published generation, artifact/compiler identity,
bindings, collision kind/owner/exact fingerprint, completion state, and tool
health without prompts or secret reads. The dry-run CAS values can be copied
directly into the reviewed `--apply` invocation. `test` verifies compiler
integrity and installed native parsers; repository native fixtures own exact
argv/exit and lifecycle semantics. No flag bypasses
security ceilings, risk acknowledgement, secret/path safety, native ownership,
or schema checks, and no command executes an action/provider.


## Built-in DevOps packs

The current local implementation contains 11 reviewed static packs with 33 typed actions. Packs are convenience catalogs, not hidden automation: every action is disabled by default, has a stable identity and documentation, declares its tool/version and risk requirements, and must pass collision/health checks before alias eligibility. Tool discovery is read-only; provider processes are not started on keystrokes, startup, rendering, or normal action search.

## Native imports and trusted workspace tasks

Selected native aliases can be imported only from explicitly supported inventories and only after a bounded dry run shows exactly what will be created. Workspace task bridges are narrower still: Automexia exposes explicit task names from reviewed task runners (for example `just`, Task, or `mise`) only after the workspace is trusted with a path-free digest/revision receipt. Trust is revocable, runtime authorization is rechecked, and a changed workspace cannot inherit stale approval.

These bridges are insertion-oriented. They do not turn arbitrary project files into background code execution, and they do not make provider authentication or secret stores part of completion/search.

## Security and privacy rules

- Treat terminal text, imported aliases, workspace files, provider-generated completion output, and shell profiles as untrusted input.
- Never derive executable commands from rendered cells or copy terminal output into a command without explicit review.
- Keep completion/provider generation off the keystroke, renderer, VT, and PTY paths.
- Use exact argv/process launch boundaries rather than `sh -c`, `cmd /c`, or string interpolation when execution is eventually enabled.
- Keep secrets out of canonical action definitions, generated aliases, logs, receipts, completion caches, and diagnostic JSON.
- Reject unsafe links/reparse points, path traversal, oversized artifacts, tampered digests, stale revisions, and incomplete transactions.
- Preserve native-shell fallback whenever managed state is missing, invalid, unsupported, or untrusted.

## Performance and resilience

Interactive search and completion operate on bounded in-memory/private data. Slow provider discovery, authentication, network requests, and filesystem scans are never allowed on every keystroke. Worker results carry a generation/session identity so stale work cannot publish into another session. Managed caches use explicit limits, last-known-good state survives a failed refresh, and uninstall removes only exact Automexia-owned artifacts.

## Verification

For shell integration, verify `TERM_PROGRAM=Automexia` and `AUTOMEXIA_SHELL_INTEGRATION=1` inside a managed session, then confirm context/path metadata appears before the first keypress and changes after a directory change. Repository-focused shell checks include the native PowerShell/CMD jobs on Windows and Bash/Zsh/Fish integration jobs on Unix hosts. Completion and alias state should be inspected with `doctor`, `preview`, and `test` commands before any `--apply` operation. The complete merge/release gate remains `cargo ready`; see [Testing and release](../developer/testing-release.md).

## What is intentionally not claimed

Automexia does not claim that saved Quick Actions can execute with arbitrary authority, that third-party plugins may run inside the terminal process, that completion should contact cloud providers while typing, or that all shell ecosystems have identical semantics. Those are separate capabilities with separate review and release gates. See [Architecture](../developer/architecture.md) and [Roadmap](../project/roadmap.md).
