# Terminal maintenance requirements

Status: technical correction requirements, not a statement that fixes have
shipped. This document covers ordinary terminal reliability and compatibility.
It contains no product delivery schedule or commercial specification.

The source inspection for this document used the implementation at
`3cb729af27`. Recheck the owning code and tests at the exact revision being
fixed. Active development can change an implementation without closing a
reported native-workflow failure.

## How to use this document

Use [Architecture](ARCHITECTURE.md) for ownership, [Testing](TESTING.md) for
evidence policy, and [Terminal interaction requirements](TERMINAL-INTERACTION-REQUIREMENTS.md)
for keyboard discovery, customization, overlays, and local previews.
[Features](FEATURES.md) remains the current availability authority. Existing
user references continue to describe current behavior until code changes.

For each applicable requirement below:

1. Record the reported symptom, exact revision/package, platform, shell, input
   mode, and smallest redacted reproduction.
2. Classify the existing path as implemented, partial, missing, or externally
   unverified using source and tests, not this checklist alone.
3. Add a regression through the real owning path before changing behavior.
4. Extend that owner; preserve implemented safeguards and unrelated work.
5. Rerun affected correctness, security, visual, resource, native, and recovery
   checks. Record unrun environments explicitly.
6. Update the current guide/reference and feature evidence only after the
   resulting implementation and evidence agree.

A reported failure remains open even if related unit tests pass. A requirement
marked for investigation is not a confirmed diagnosis.

## Source and evidence map

Unprefixed application paths below are relative to
`apps/automexia-terminal/src/`; crate-prefixed paths are repository-relative.
Test owners name places to extend, not tests
claimed to have been executed for this document.

| Requirement | Existing source owner | Existing test/evidence entry point | Inspection disposition |
|---|---|---|---|
| R1 Reflow and retained content | `rio-vt/src/crosswords/grid/resize.rs`, `rio-vt/src/selection.rs`, application `context/renderable.rs` | Grid/selection tests; resize-stress and conformance suites | Reflow and remapping exist; reported end-to-end failure still needs reproduction |
| R2 Damage and command highlighting | `context/renderable.rs`, `grid_emit.rs`, `renderer/command_results.rs` | In-module damage/command-result tests; [command-result assurance](COMMAND-RESULT-ASSURANCE.md) | Damage merging and anchors exist; verify the complete render path |
| R3 Paste and pointer targeting | `screen/mod.rs`, input routing and session messenger | Screen/input tests; native multi-pane clipboard workflows | Current paste writes through the current context; gesture-to-target invariance needs proof |
| R4 Native shell keys | `bindings/mod.rs`, `bindings/registry.rs`, `automexia-keybindings/src/` | Binding collision/registry tests and actual shell input | Confirmed default Ctrl+R/Ctrl+D collision, including hard-coded palette labels |
| R5 Session exit and cleanup | `context/mod.rs`, `context/launch.rs`, `teletypewriter/src/` | `teletypewriter/tests/pty_lifecycle.rs`; native lifecycle suites | Owners exist; preserve recent EOF fixes and verify hidden-session paths |
| R6 Responsiveness | Session launch, PTY queues, snapshot publication, platform presentation | Same-host interactive benchmarks; native WSL/ConPTY and Unix PTY suites | User-reported latency; bottleneck not established by source inspection |
| R7 Unicode, control strings, images | `rio-vt/src/`, `sugarloaf/src/`, `grid_emit.rs` | VT conformance/property/fuzz and image-rendering suites | Existing parsers/renderers; evaluate gaps without claiming wholesale absence |
| R8 Platform and upstream maintenance | Existing native adapters and engine modules | Native platform, package, assurance/checker tests | Review individual differences; no blanket engine replacement |

## R1: Preserve content and anchors during resize

### Required behavior

Narrowing and widening the normal screen must preserve retained logical text,
hard line breaks, and soft-wrap provenance. Reflow must not manufacture spaces,
duplicate rows, drop retained characters, or join independently terminated
lines. Content already evicted by the configured history limit is not promised
to be recoverable.

A bottom-pinned viewport stays at the output tail. A user scrolled into history
keeps the corresponding logical anchor where retained, rather than being
silently returned to the bottom. Cursor, selection endpoints, search locations,
command decorations, and image placements must follow the same authoritative
remapping. If an anchor is evicted, invalidate or clamp it by an explicit
documented rule; never reuse its old physical row for unrelated content.

Normal and alternate screens need separate contracts. Full-screen applications
own their redraw and do not inherit normal-screen scrollback/reflow semantics.
A resize cannot resurrect discarded alternate-screen content.

### Implementation approach

Extend the existing `Grid::resize` and `ReflowRemap` path. Keep logical
terminal positions separate from pixel rectangles and render-cache indices.
Apply the final accepted resize generation consistently to PTY dimensions,
terminal state, viewport mapping, and presentation. Coalesce obsolete resize
requests without losing the final size or reordering input/session lifecycle.

A second scrollback buffer in the renderer, reparsing rendered text, or
restoring pixels from a screenshot would create competing authorities and is
not an acceptable fix.

### Acceptance and regression evidence

- Feed exact bytes through the parser, then exercise widths such as
  180, 120, 80, 62, 40, 100, and 180 cells with multiple heights.
- Compare independently specified retained logical text, hard/soft breaks,
  cursor and selection results before and after the round trip.
- Include wide characters, combining sequences, ANSI styles, tabs, blank lines,
  a partly written final line, images, history eviction, and wrapped prompts.
- Exercise output during resize, rapid split creation/closure, scrolled-up
  views, normal/alternate transitions, and unrelated sibling panes.
- Verify parser-to-grid-to-viewport-to-draw-data-to-pixel results. A test that
  injects final grid cells alone does not reproduce a parser/reflow failure.
- Require no stale geometry, cross-pane changes, unbounded retained maps, or
  regression in interactive latency and cleanup.

## R2: Make damage, style, and command highlights converge

### Required behavior

Completed-command styling must stay attached to the same retained command
identity and owning session across reflow, scrollback movement, and redraw.
Width-dependent line numbers or cached pixel offsets are not sufficient
identity. Missing, malformed, or late shell metadata must not decorate a
different command.

Every visible mutation must eventually be painted, including removal of a
highlight, cursor-only changes, erase operations, theme/font updates, selection,
search, and image deletion. The final screen must converge when output becomes
quiet; requiring another keystroke, focus change, or resize to repaint is a bug.

A retained snapshot must keep its original style interpretation. Recycling a
style identifier for a newer snapshot must not recolor an older live snapshot.
Bold, faint/dim, inverse, default foreground/background, and explicit ANSI
backgrounds remain independently correct. Disabling decorative color must not
remove readable status or change copied terminal text.

### Implementation approach

Extend the existing command-result anchors/identities, pending-damage merge,
grid emission, and renderer style ownership. Publish updated state before wake,
preserve damage until the appropriate consumer has observed it, and ensure
in-flight event suppression cannot hide the final update.

Use ownership-aware style retention or an equally bounded immutable mapping;
reclaim only when no live snapshot references the values. Do not invent a new
command-execution engine or a second terminal history to repair decoration.

### Acceptance and regression evidence

Compare the final incremental frame with an independently accumulated reference
screen/full redraw using the same public fixture input. Test mutation during
render, wake coalescing, no-op frames, clear/erase, cursor motion, scroll,
reflow, style reuse, delayed metadata, theme changes, and route replacement.
Require exact controlled raster equality and no permanent repaint loop after
quiescence. Track cache growth and final release over repeated sessions.

## R3: Preserve paste intent and exact input ownership

### Required behavior

A pointer-triggered paste targets the pane selected by that gesture; a keyboard
paste targets its focused input owner. Capture route and session generation
when accepting the action. Clipboard delays, focus changes, local-tab switches,
or pane replacement must never redirect the payload to a different session.
If the original target is gone, cancel rather than choose the new current pane.

Search, palette, and other active input surfaces keep their own paste handling.
Terminal input must not leak underneath an overlay. Existing selection-copy
behavior, right-click copy/paste semantics, primary-selection behavior, and
terminal mouse reporting remain compatible unless separately reviewed.

The target's applicable input mode determines bracketed-paste framing. Preserve
one ordered paste operation so normal keystrokes or another paste cannot split
its start, payload, and end boundaries. Bound payload bytes, queueing,
cancellation, and any chunking without splitting an encoded character.

### Current behavior and important limitation

`Screen::paste` currently removes ESC and Ctrl+C from bracketed payloads and
normalizes line breaks to carriage returns for its normal paste path when the
child has not enabled bracketed paste. Preserve these established protections
until a separately tested compatibility/security change is accepted.

“Paste does not add Enter” does not mean pasted multiline text cannot execute.
A shell without bracketed-paste support can interpret existing pasted line
breaks as input submission. Any warning or confirmation must describe that
limitation honestly; it cannot claim a universal non-execution guarantee.

### Pointer, links, and IME

Use the same pane transform for paint, hit testing, selection, mouse reporting,
cursor, and IME placement. A link gesture must retain press/release identity,
cancel on drag or target replacement, and respect terminal mouse ownership.
Inspect and validate a supported link on explicit activation; terminal output
alone does not open it. Keep existing host-override behavior discoverable.

### Acceptance and regression evidence

Use two real PTYs with a byte-recording fixture. Start paste on one pane,
change focus or close it before completion, and assert exact recipient bytes
and zero bytes in every other pane. Repeat across local/window tabs, overlays,
selection, bracketed and plain modes, CR/LF/CRLF, Unicode, hostile delimiters,
large bounded payloads, and shutdown. Add actual supported-shell/editor/TUI
runs: the byte fixture proves transport, not the shell's interpretation.

## R4: Restore native shell control keys by default

### Confirmed source conflict

`clone_split_key_bindings` assigns bare Ctrl+R and Ctrl+D to pane cloning.
It supplies Ctrl+Alt alternatives for the displaced shell bytes.
`renderer/command_palette.rs` repeats the clone chords in constants and tests.
That behavior is accurately listed in [Keyboard](KEYBOARD.md); it is not
already fixed by the typed registry's existence.

The `automexia` registry builder can return no compiled registry when there
are no typed user bindings. A change to bundled typed profiles alone therefore
cannot prove that the legacy defaults are corrected.

### Required correction

Remove the default interception of bare Ctrl+R and Ctrl+D for cloning. Keep
clone actions available through the palette and explicit user bindings.
Leaving an action unbound is preferable to introducing another unreviewed
collision. Preserve unrelated fresh-split, tab, search, and platform shortcuts.

Protect conventional shell control input, including Ctrl+C/D/R/Z/A/E/U/K/W/L/S/Q,
from new default application interception. This is an input-ownership rule,
not a claim that every shell assigns identical meanings. Preserve documented
mode exceptions such as selection-aware copy, search, Vi mode, and explicit
user overrides. Native terminal/line-discipline behavior remains authoritative.

Use the existing action IDs, profile compiler, scope predicates, binding origin,
and legacy unbind bridge. Palette badges and shortcut help must describe the
effective selected profile and user overrides, including “unbound,” rather
than hard-coded default strings.

If a user deliberately overrides a protected chord, show the collision and
require explicit acknowledgement in any binding editor. Cancelling changes
nothing. Do not silently rewrite an intentionally configured shortcut during
migration; distinguish inherited defaults from user-owned entries.

### Acceptance and regression evidence

Test default config with no typed bindings, explicit `automexia`, every
supported compatibility profile, legacy bindings, typed bind/unbind,
invalid reload, reset, precedence, multiple surfaces, and mode transitions.
Assert exact child bytes and absence of split creation for restored chords.

Run actual Bash, Zsh, Fish, and PowerShell/PSReadLine configurations that the
release claims; name editing modes and any user overrides in redacted evidence.
Readline's Emacs-mode backward history search is one concrete native behavior,
not a universal shell mapping.
[GNU Bash history search](https://www.gnu.org/software/bash/manual/html_node/Commands-For-History.html)
and [PSReadLine key inspection](https://learn.microsoft.com/en-us/powershell/module/psreadline/get-psreadlinekeyhandler)
provide the relevant shell authorities.

## R5: Close the correct session and release its resources

An exit event identifies its session and generation, including background
window tabs and hidden pane-local tabs. Resolve it across the owning context
structure; do not remove whichever tab happens to be current. Update lookup
indices, visibility, focus, and renderer resources consistently. A stale exit
must not close a newly created session that reused a slot.

Use the existing PTY/process owner for cancellation, input shutdown, output
draining, exit publication, joining, and descendant cleanup. Preserve already
implemented EOF normalization and regression tests; a source-present upstream
fix is not a reason to apply it again.

Unix process-group operations need verified owned identities and must never
target a broad or reused process identifier. Windows handles and applicable
Job Object/pseudoconsole ownership need native evidence for the actual child
tree. Do not promise termination of detached processes outside owned scope.

ConPTY I/O and teardown must avoid blocked-channel deadlocks, including final
output produced while closing. Microsoft's
[pseudoconsole lifecycle guidance](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session)
describes the channel and handle responsibilities; adapt it to existing owners
rather than introducing a second I/O stack.

Test foreground/background exit, startup failure, immediate close, simultaneous
exit/resize/input, pane/tab replacement, repeated open/close, large final output,
and application shutdown. Independently verify surviving sibling identity,
exact final status/output policy, zero owned child/handle/thread leaks, and no
late result publication. Bound drain/termination deadlines and report any
output truncation rather than waiting forever.

## R6: Diagnose and correct responsiveness on the owning path

### Measure before choosing a fix

Separate user-visible startup milestones: accepted launch, process creation,
PTY readiness, first bytes, shell integration readiness, first prompt, and
first editable frame. Separate WSL cold-VM startup from a warm distribution and
from ordinary local shell startup. Record shell/profile, terminal size,
renderer/backend, sample count, and same-host baseline without private paths
or terminal content.

For input latency, measure input acceptance through PTY write, child response,
PTY read, parse/state update, snapshot publication, wake, and presentation.
Report distributions and outliers, not only mean throughput or a parser
microbenchmark. A missing shell prompt marker is “unavailable,” not zero time.

### Candidate corrections to validate

- Move repeated filesystem, distribution discovery, and optional initialization
  off input/resize/render paths using existing bounded workers.
- Inspect the frequency and cost of WSL distribution validation. Cache only
  validated bounded discovery with explicit freshness/invalidation; preserve
  fail-closed behavior for missing or changed launch targets.
- Never silently fall back from the requested WSL distribution or shell to a
  different shell to make startup look faster.
- Keep ordered input, resize coalescing, fair PTY scheduling, bounded parsing
  batches, publish-before-wake, and cancellation visible in measurements.
- Verify readiness semantics when a bounded read stops before draining a Unix
  descriptor; handle partial writes and control messages without lost wakeups,
  unnecessary writable polling, busy loops, or starvation of another pane.
- Keep retries bounded and error-specific. Do not turn an invalid launch into
  an indefinite retry loop or hide a failed first sample.

Choose only corrections supported by profiling and regression evidence.
Increasing queue sizes, adding threads everywhere, replacing the runtime,
disabling validation, or copying a faster benchmark from another OS is not a
performance diagnosis. Do not publish speculative millisecond targets as
measured achievements.

### Acceptance and regression evidence

Use cold/warm launch, idle typing, sustained output, mixed interactive/output
panes, resize storms, long scrollback, active search, previews, and shutdown.
Compare same-host before/after latency, CPU, memory, handles, workers, queues,
and cleanup. State backend, OS/architecture, shell, sample method, noise bounds,
and remaining external platforms. A Unix improvement does not prove a WSL or
ConPTY improvement.

## R7: Keep text and graphics compatibility coherent

### Unicode and selection

Use the existing text/VT/rendering owners and explicitly record the supported
Unicode data version and terminal width policy. Unicode grapheme segmentation
and terminal cell width are related but different contracts; adopting a
segmentation algorithm alone does not prove correct display widths.

Test combining sequences split across reads, emoji variation selectors and
joiners, regional indicators, CJK, zero-width marks, fallback fonts, bidi
controls, selection, copy, search, and reflow. Prevent cluster fragments,
invisible selection drift, and disagreement between draw positions and hit
testing. Document any intentionally unsupported behavior; advertise protocol
modes only when the complete implemented behavior is tested.
[Unicode text segmentation](https://www.unicode.org/reports/tr29/) is a
reference for segmentation, not a replacement for terminal-specific tests.

### Control strings and synchronized output

Preserve parser state correctly across arbitrary read boundaries. Cover
malformed, cancelled, truncated, nested-looking, and oversized CSI/OSC/DCS/APC
input and recovery to normal text. Synchronized-output handling must not allow
an untrusted child to freeze presentation indefinitely; use the existing
bounded timeout/recovery owner.

Compare modern and legacy encodings against the actual implemented parser
before accepting an upstream compatibility patch. Do not announce support for
an encoding or extension merely because it appears in another terminal.

### Images, color, and fonts

Protocol images and local quick-look images retain separate identities,
authorities, limits, and lifetimes. Verify terminal scroll/erase/delete,
normal/alternate transitions, resize, route closure, late uploads, CPU cache
eviction, atlas/GPU release, and transparent pixels. A freed CPU reference
alone does not prove texture cleanup.

Check explicit foreground/background colors, inverse/bold/dim combinations,
background alpha, selection contrast, synthetic bold/italic, fallback fonts,
and malformed font/image inputs. Reuse existing decoders and platform
adapters; keep file/network authority out of the parser and renderer.
[Image previews](IMAGE-PREVIEWS.md) owns actual supported formats and limits.

## R8: Reconcile upstream and platform fixes selectively

Review each proposed upstream change against its exact revision, full patch,
local divergence, license/notices, dependencies, affected tests, and native
platform applicability. Classify it as already present, applicable adaptation,
superseded, inapplicable, or unresolved. Preserve the final corrected upstream
series rather than importing an early patch that later needed repair.

Retain attribution for inherited code even where the surrounding engine uses
another license. A renamed file or rewritten module boundary does not erase
notice obligations. Do not update the dependency/toolchain baseline as an
incidental part of a terminal fix.

Native candidates include macOS IME/focus and resize behavior, Windows
startup/background and ConPTY lifecycle, and X11/Wayland compositor
capabilities. Optional visual effects must degrade without breaking basic
terminal rendering. Validate supported OS-version symbols and backend-specific
fallbacks instead of assuming one platform fix transfers unchanged.

Build and assurance improvements must follow
[Development cache](DEVELOPMENT-CACHE.md): separate mutable compiler targets
from verified tools, preserve active leases and dirty worktrees, preview exact
cleanup candidates, and never weaken a test or scan for speed. The local
pre-push hook remains dormant; this document does not reactivate it.

## Completion record for a technical fix

A maintenance review must record:

- requirement and observed failure; classification before and after;
- exact source revision and package identity, owner and affected call sites;
- failing real-path reproduction and independent expected result;
- focused, interaction, property/fuzz/concurrency, and checker-mutation results;
- geometry, exact controlled raster, native input and accessibility evidence;
- same-host performance/resource/storage results and final cleanup;
- security, cancellation, stale-result, disable/recovery/rollback coverage;
- supported native environments and every unavailable external gate;
- current guide, reference, architecture, and assurance entries updated; and
- remaining limits without claiming universal correctness.

The matrix in this document is a maintenance checklist, not a substitute for
the versioned [feature-test reinforcement](FEATURE-TEST-REINFORCEMENT.md) ledger.
No row becomes complete merely because its documentation was written.
