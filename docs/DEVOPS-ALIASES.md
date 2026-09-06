# DevOps aliases and shell projections

Status: CP2.0-CP3.3 source foundations are implemented; activation and release
claims remain limited by the current feature catalog and native evidence.

Implementation owner: `automexia-command-productivity`. This package name is a
current source-maintenance fact, not a commercial roadmap.

## Product outcome

Automexia can turn reviewed local actions into optional native-shell aliases
without replacing the shell editor, parser, completion engine, configuration,
or execution authority. Nothing short is enabled by default.

## Non-negotiable principles

- The user previews and explicitly activates every generated artifact.
- Selecting, importing, compiling, or inserting an action never sends Enter.
- Native definitions win; Automexia never silently overwrites a user alias.
- No secret aliases and No hidden context mutation are permitted.
- No first-party pack alias is activated by default or shipped implicitly by CP3.2.
- Disable, rollback, regeneration, and removal remain available without a
  working network or provider account.

## Architecture and ownership

The native shell owns parsing, expansion, quoting, cursor state, completion,
history, startup files, and final execution. Automexia owns only validated
action records, deterministic candidate projections, and bounded files under
its own configuration root. Optional discovery remains off renderer, input,
PTY, resize, and startup hot paths.

CP3.0 defines the pure projection compiler. CP3.1 adds persistent opt-in user
aliases. CP3.2 covers reviewed first-party static packs. CP3.3 covers bounded
native imports and trusted workspace task bridges. These layers do not create
provider, credential, network, or task-execution authority.

## Canonical data model

Every record has a stable identifier matching `[a-z][a-z0-9-]{1,31}`, a
display label, scope, provenance, revision, collision policy, and one explicit
template:

`template: TypedArgv(executable_id, ArgumentToken[]) | RawInsertOnly(shell, text)`

Projection behavior is explicit:

`mode: Auto | CommandAlias | WrapperFunction | FishAbbreviation | DoskeyMacro`

Arguments use one declared rule:

`argument_policy: None | ForwardAll | TypedBindings`

Raw text is insertion-only. It cannot become a structured executable action or
gain automatic execution through projection.

## Persistence and cross-session behavior

User-private actions are stored transactionally in `actions/actions.toml`, with
the last valid predecessor retained as `actions/actions.previous.toml`.
Generated candidates live under `generated/aliases`; the application never
writes directly to arbitrary shell profile files.

Publication uses temporary-file validation, atomic replacement, directory
sync where supported, and pointer-last commit ordering. Writers use
compare-and-swap against the reviewed revision so a stale preview cannot
replace newer data. Corrupt or partial state fails closed to the last-known-good
record and a redacted diagnostic.

## Scope, precedence, and collision handling

Precedence is explicit and deterministic: native shell definitions remain
authoritative; user-private Automexia actions precede opt-in static-pack
candidates; imported candidates never silently replace either. Collision
reports identify both sources without exposing command secrets or local paths.

Activation requires a reviewed destination, exact revision, and explicit apply
step. Refuse ambiguous identifiers, reserved shell words, unsupported quoting,
stale revisions, symlink/reparse-point escapes, or destinations outside the
owned root.

## Shell-specific projections

- PowerShell 5.1 and PowerShell 7+ use functions only when forwarding or typed
  binding cannot be represented safely as a simple alias.
- Bash and Zsh use deterministic functions or aliases with shell-native quoting.
- Fish prefers functions or abbreviations according to the declared mode.
- CMD uses a bounded DoskeyMacro projection when its semantics are sufficient.
- WSL is treated as a distinct Unix-shell environment with its own owned output;
  Windows and WSL startup files are never assumed to be interchangeable.

Unsupported or lossy projections are rejected rather than approximated with
shell evaluation. Native shell parsers are the independent syntax oracle.

## Completion linkage

Alias names may be exposed to the source-owned native completion adapter only
after validation and activation. Completion suggestions cannot activate an
alias, change context, invoke a provider, or execute a command. See
[shell productivity](COMMAND-PRODUCTIVITY.md) and
[shell integration](SHELL-INTEGRATION.md).

## User experience

The review surface shows identifier, scope, provenance, exact destination,
collision result, projection mode, revision, and rollback availability.
Health states include `Ready`, disabled, stale, invalid, unsupported, and
recovery-required. Keyboard focus returns to the originating pane after close.

CP2.2 Quick Actions search is bounded and deterministic. Choosing an item offers
Insert without Enter or copy; exact launch is never inferred from a label.

## Built-in first-party DevOps packs

CP3.2 permits reviewed, versioned, static, free packs for ordinary tools only.
A pack contains typed or raw-insert templates, provenance, compatibility data,
and collision metadata. It cannot read live infrastructure, authenticate,
contact a provider, or mutate context merely by being installed or searched.

No first-party pack alias is activated by default or shipped implicitly by CP3.2.
Users enable individual candidates after review. Pack removal preserves native
and user-created definitions.

## User-created aliases and import/export

CP3.1 - persistent opt-in user aliases uses the private local action store and
the same review, revision, projection, activation, rollback, and removal rules.
Export redacts secrets and machine-local metadata by default.

CP3.3 - native imports and trusted workspace task bridges accepts only bounded,
explicitly selected input. Imports are candidates, not active aliases. Native
inventory and workspace recipe parsing are capability-separated adapters; the
core compiler receives validated records only.

No native inventory, task discovery, recipe parsing, provider process, network, credential read, or task execution
occurs on search, compilation, preview, or completion paths.

## Existing tools and open-source projects

Automexia interoperates with native PowerShell, Bash, Zsh, Fish, CMD/Doskey,
and WSL mechanisms instead of inventing a replacement shell language. Existing
user tools such as shell profile managers and dotfile repositories remain
authoritative; generated files are optional, isolated, and removable.

## Security and privacy

Treat imported files, action metadata, labels, templates, paths, shell output,
and provider text as untrusted. Enforce byte/count/depth limits; reject controls
and bidi ambiguity in UI labels; preserve typed argument boundaries; prohibit
implicit Enter and shell-evaluated fallbacks; redact diagnostics; and never
persist credentials, tokens, hidden environment values, terminal history, or
machine-local identifiers in exportable artifacts.

Symlink, reparse-point, traversal, stale-revision, duplicate-key, corrupt-file,
read-only, disk-full, interrupted-write, and concurrent-writer cases fail
closed. Optional projection failure cannot break an ordinary terminal session.

## Performance, resilience, and resource budgets

- Search target: `<= 25 ms p95 off renderer/input/PTY paths`.
- Pure projection target: `<= 16 ms p95; deterministic and allocation-bounded`.
- Post-warmup refresh target: `<= 50 ms p95 post-warmup; no provider, network, action execution, or per-alias subprocess`.
- File size, record count, diagnostic count, queue depth, concurrency, retries,
  and retained predecessors are bounded by source-owned policy.
- Cancellation and stale-generation rejection apply before publication.

Repeated validation includes `1,000 save/regenerate/reload cycles` and proves
`no handle/task/file/storage growth` beyond documented bounded caches and the
single retained predecessor.

## Accessibility and responsive UX

All review, collision, activation, recovery, and removal flows are keyboard
operable with visible focus and focus restoration. Status never relies on color
alone. Verify narrow through 8K layouts, high contrast, reduced motion,
localization, Unicode, long labels, IME, and `200%/400% text scale`.

Native evidence includes Narrator/NVDA on Windows, VoiceOver on macOS, and
AT-SPI/Orca on supported Linux desktops. Automated semantics do not replace
human assistive-technology evidence.

## Verification plan

Test zero, one, boundary, maximum, over-limit, repeated, malformed, fragmented,
Unicode, control/bidi, collision, stale, concurrent, cancelled, disabled,
rollback, removal, restart, and cleanup cases. Independent oracles compare
canonical records, exact generated bytes, native parser results, filesystem
trees and digests, argument vectors, and forbidden side effects.

Run the repository-owned command-productivity, alias-spec, documentation,
architecture, confidentiality, and full validation gates. Native tests cover
PowerShell 5.1 and PowerShell 7+, Bash, Zsh, Fish, CMD, and WSL on each claimed
platform; cross-compilation alone is not native evidence. See
[testing](TESTING.md#verification-plan).

## Delivery phases

- CP2.0 - contract and fixtures (implemented)
- CP2.1 - user-private action store (implemented foundation)
- CP2.2 - Quick Actions search, editor, and insert/copy source boundary
- CP3.0 - deterministic shell projection compiler
- CP3.1 - persistent opt-in user aliases
- CP3.2 - reviewed first-party DevOps packs
- CP3.3 - native imports and trusted workspace task bridges

These labels describe source and assurance ownership. They do not disclose
private commercial sequencing or make an unsupported availability claim.

## Acceptance criteria

The public contract is met only when native shell authority is preserved,
generated bytes are deterministic, no implicit execution is possible, unsafe
input fails closed, lifecycle operations are reversible, optional failure does
not affect terminal I/O, resource bounds hold, documentation matches source,
and every claimed native environment has current evidence for the exact build.

## Primary references

- [ADR 0015](adr/0015-shell-native-completion-and-typed-quick-actions.md)
- [Architecture](ARCHITECTURE.md)
- [Command productivity](COMMAND-PRODUCTIVITY.md)
- [Compatibility](COMMAND-PRODUCTIVITY-COMPATIBILITY.md)
- [Threat model](COMMAND-PRODUCTIVITY-THREAT-MODEL.md)
- [Shell integration](SHELL-INTEGRATION.md)
- [Testing](TESTING.md#verification-plan)

Unreleased action collections, provider workflows, organization features, and
commercial packages remain private and are not part of this specification.

## CP3.0 projection compiler baseline

The pure compiler converts validated action records into deterministic,
shell-specific candidate artifacts. It has no activation or execution authority.

## CP2.2 action search editor and insert/copy

Quick Actions search returns bounded reviewed results for copy or insertion
without Enter; labels never imply an executable launch contract.

## CP3.1 persistent opt-in user aliases

Persistent user aliases use the transactional private store and explicit
preview, activation, rollback, disable, and removal lifecycle described above.

## CP3.2 first-party static DevOps packs

Static packs remain reviewed, versioned, capability-free, and disabled by
default. Installing or listing a pack never activates an alias.

## CP3.3 native imports and trusted workspace task bridges

Imported aliases and workspace tasks remain bounded candidates. Discovery and
review never execute them or gain provider, credential, network, or PTY input
authority.
