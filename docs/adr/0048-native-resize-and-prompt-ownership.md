# ADR 0048: Native resize and prompt ownership

Status: Accepted for the requested resize-preservation fix; platform validation
remains separately reported.

## Context

Live ConPTY regression tests reproduced completed output being claimed by the
active prompt and subsequently removed by prompt repair. Growing the VT viewport
also pulled history back into native repaint coordinates. A wrapped logical line
split at the native scrollback boundary needs both halves preserved independently.

## Decision

The existing core VT/grid remains the sole output owner. The PTY adapter selects
an explicit resize policy before reading output. Windows ConPTY sessions, including
WSL, preserve the mutable viewport origin; Unix PTYs and standalone VT consumers
retain their existing reflow policy. Escape sequences cannot select this policy.

Fixed-size point tracking includes the native viewport origin. A native wrapped
seam reflows its retained prefix separately from the native suffix. Unused tail
cells have an explicit non-content flag, retain ordinary wrap ownership, and are
excluded from logical text, copy and search. Future reflow removes that padding
before joining cells. No duplicate output buffer, persistent cache, shell-text
heuristic, new dependency or extension is introduced.

Prompt editing alone does not grant ownership of arbitrary repaint rows. Erases
of historical output cannot arm prompt reconstruction. Existing context in
scrollback is not considered missing merely because it is outside the viewport.
Native resize failures return errors through the existing retryable performer
boundary instead of panicking.

Live-worker burst tests additionally reproduced different effective resize
sequences: the UI reflowed every requested size while the worker coalesced native
sizes. For managed ConPTY sessions the worker must commit both grid and native
geometry from the same coalesced message. UI resize requests retain only current
cell metrics; they do not pre-reflow the native grid. The worker drains available
old-size output before applying queued sizes and publishes damage after the grid
commit. Failed native resizes retain the old grid and remain retryable. Standalone
VT and Unix paths keep their synchronous resize contract. All current PTY-backed
callers already send the native size after requesting core resize.

This remains one grid and one worker, not a second snapshot or output cache.
Pending cell metrics have constant storage. Renderer snapshots may briefly retain
the last committed grid during a resize, then refresh on the worker's damage event;
no requested dimensions authorize shell output to resize the application window.

### Native hard-line fill during table resize

The long-table live fixture reproduced an extra wrapped row before the first
native repaint. ConPTY can serialize a hard line with trailing spaces to the old
viewport width. Its reflow measures through the last non-space glyph, whereas
Unix explicit spaces remain content. Reflowing native fill as content displaced
the live/history boundary; the subsequent repaint duplicated filename fragments.

Only the ConPTY column-reflow passes trim hard-line trailing fill. Forced wraps
keep their full width, leading/interior spaces and Unicode extras are retained,
and the cursor reserves its existing distance through blank cells. The existing
grid remains the owner; no output cache, shell-name detection, extra worker or
resize delay is added. Unix and alternate-screen no-reflow contracts are unchanged.
Native fill attributes are not allowed to create additional logical output rows.

This follows the distinction in Microsoft's
[row measurement](https://github.com/microsoft/terminal/blob/main/src/buffer/out/Row.cpp)
and [text-buffer reflow](https://github.com/microsoft/terminal/blob/main/src/buffer/out/textBuffer.cpp).
The independent replay uses the observed fictional native repaint, fragmented at
every byte boundary. Live fixtures retain 32 long table rows, exact ordering,
duplicate rejection and prompt adjacency through raw adapter, worker burst and
intermediate-commit paths. The old eight-short-row fixture alone was insufficient.

A no-resize control also proves that a silent fixture probe preserves rows and
cursor position. CMD's former SET /P plus Enter probe inserted a native newline;
it could shift the viewport while resizing. Non-echoing keys now acknowledge
unchanged output, while a separate worker test retains real Enter coverage.
Final child release uses a separate line read only after viewport assertions,
so cleanup cannot repair the state being verified or lose PAUSE typeahead.

### Editing after a sibling closes

ConPTY prompt restoration preserves the protocol cursor and pending-wrap state.
Decorative context recovery must not change the origin of subsequent relative
VT moves or cursor-position reports, including between fragmented erases.

The worker now stops channel consumption at input until its bytes reach the
existing PTY write adapter. Merely queuing those bytes does not authorize a later
resize to overtake them. Each channel batch has a 128-message fairness bound.

After a successful Windows character-grid change, input has a 50 ms settle
deadline. PSReadLine 2.4.5 skips resize reconciliation inside its 50 ms render
fast path; live ConsoleHost tests reproduced its old input row after a silent
shrink/grow. The worker continues output, child-exit and cancellation processing
without sleeping or adding a thread. Ordinary input, Unix PTYs, failed resizes,
duplicates and pixel-only changes do not start this deadline. This is an explicit
post-resize latency tradeoff, not a throughput improvement. No shell key is
injected, module replaced, profile edited or private editor state accessed.

Machine channels retain one separate cancellation readiness registration, shared
by sender clones. Shutdown wakes and retires the worker even when unsent input
and later resizes fill the ordinary channel. This uses the existing poller with
no helper thread, polling timer, queue copy or extra process authority. The
application messenger and embedding surfaces preserve this paired sender;
dead/recording contexts may adapt ordinary channels, which still receive the
original shutdown message.

Fake-clock tests cover expiry between polling and write registration and pending
input. Separate worker/poller tests cover cancellation. Native tests separately
verify editor text, native/VT cursor positions and successful cleanup. Revisit
the compatibility deadline when native editor behavior no longer needs it.
Windows pipe adapters retain exactly one join owner after I/O failure: if error
delivery has already joined and removed the worker, destruction does not take
that handle again. Native closed-peer tests cover both read and write adapters.
This stays in the existing process adapter, not the renderer or an extension.
Primary evidence: [PSReadLine render optimization](https://github.com/PowerShell/PSReadLine/blob/v2.4.5/PSReadLine/Render.cs)
and [interactive input lifecycle](https://github.com/PowerShell/PSReadLine/blob/v2.4.5/PSReadLine/ReadLine.cs).

## Multi-column history seams and exact cursor margins

Native WSL/eza tests reproduced lost inter-column spaces when a wrapped row
crossed the history/live seam. Temporarily clearing its wrap flag allowed
hard-line padding removal to discard real column cells. The existing reflow
passes now partition at that seam without clearing its wrap identity and track
its final content cell alongside selection and viewport points. Only newly
added fill is marked as non-content padding. This adds one fixed-size, frame-local
point, no persistent cache, worker, process, per-cell identity or dependency.

A second native failure occurred when the cursor column exactly equalled the
new width. ConPTY reserves the blank cell under its cursor and wraps it onto the
next row; substituting Unix delayed-wrap behavior changed the native viewport
origin and let repaint overwrite adjacent text. The native policy now includes
that exact-boundary cell; Unix delayed-wrap behavior remains unchanged.
Copying all-blank soft-wrap fragments also retains their spacing instead of
inserting newlines. Intentional hard blank lines remain distinct.

The independent oracles are exact retained text, per-filename uniqueness,
original column positions, pre-resize selection, cursor position, history bounds
and controlled CPU pixels after the suffix enters history. The live WSL test
uses installed eza with 28 fictional filenames, explicit arguments and no user
profile. Raw adapter, burst and intermediate worker commits cover 48 transitions
at two viewport heights, preceded by a no-resize acknowledgment check. The
existing workspace `tempfile` library is reused only as a Windows test dependency:
explicit workspace-local roots, cleanup on unwind and checked `TempDir::close`
replace fixture-owned shell cleanup. No resolved dependency version or shipped
binary dependency changes. See the [cleanup contract](https://docs.rs/tempfile/latest/tempfile/struct.TempDir.html).

Native reference: [row width measurement](https://github.com/microsoft/terminal/blob/main/src/buffer/out/Row.cpp)
preserves forced-wrap width; [buffer reflow](https://github.com/microsoft/terminal/blob/main/src/buffer/out/textBuffer.cpp)
reserves the cursor cell and preserves the mutable viewport boundary. The fix
extends the existing core owner, not the listing adapter or an extension. Hiding
icons, caching output or rerunning user commands would not correct these contracts.

## Extreme mixed-dimension resize ordering

Fixed-seed live WSL/eza sequences exposed cases absent from independent
shrink/restore pairs. A wider but shorter viewport must join its wrapped live
rows before removing rows. Reducing height first archives a prefix which the
native repaint then duplicates. The core native policy now defers height
reduction until column reflow and live-origin reconciliation complete. Unix
policy and height-only changes retain their existing ordering.

ConPTY also paints unused rows below its cursor with spaces. Their lack of
visible content must use the same predicate as native hard-line trimming, not
the erased-cell-only predicate. Unicode extras, wide cells and intentional
prompt separators are not disposable native fill.

A former history/live seam can become entirely historical and shorter than a
later width. Extending that short row now preserves its soft-wrap marker and
marks only newly added fill as non-content. Otherwise a filename can acquire a
hard break and lose its column placement. Both column directions use the same
private row-extension mechanism; no output cache or second state owner is added.

Evidence owners include live fixed-seed sequences, deterministic byte-fragmented
native replays, exact copied text and cursor assertions, restored live/history
CPU pixels, and the correctness-checked extreme replay benchmark. The campaign
uses one- and two-cell dimensions, long fictional names with file/directory
icons, consecutive mixed-axis changes, and restoration through 512 by 96 cells.
Native desktop gestures, GPU drivers and assistive technology remain separate
gates. This is not a guarantee for every possible viewport or platform.

## Alternatives and evidence

### Native lifetime and final output

The expanded full suite exposed a teardown timeout after successful table
assertions. Its exact blocked native stack was unavailable; independent native
handle tests did reproduce leaked caller pipe endpoints and failed-attachment
pseudoconsole handles. The existing Windows adapter now retains RAII ownership,
releases caller pipe copies after child attachment, deletes initialized startup
attributes, and drains output before closing a failed startup. No new worker or
capability owner is introduced. Native output remains drained during close, as
required by Microsoft's [pseudoconsole lifecycle](https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session).

An independent terminal-lock contention test also reproduced pending output
loss on native pipe closure. The existing terminal worker now parses buffered
bytes before accepting EOF, waiting for the terminal lock on that worker only.
Windows BrokenPipe and Linux EIO use the existing platform classifier; unrelated
I/O errors still propagate. EOF does not establish child success: independent
exit and joined-cleanup assertions remain mandatory. Native handle recovery,
worker contention, live repaint and desktop pixels are separate evidence.

Disabling highlights would conceal symptoms, not recover output. Caching and
replaying shell text creates a competing authority and breaks legitimate clears.
A new extension is inappropriate for mandatory terminal state and resize.

Microsoft's legacy resize-suppression flag was evaluated but did not fix the
wrapped regression on the test host; it is not enabled. The implementation does
not depend on suppressing native repaint output or undocumented flag support.

References: [Microsoft resize API](https://learn.microsoft.com/en-us/windows/console/resizepseudoconsole),
[Windows Terminal viewport logic](https://github.com/microsoft/terminal/blob/main/src/cascadia/TerminalCore/Terminal.cpp),
[xterm.js native buffer handling](https://github.com/xtermjs/xterm.js/blob/master/src/common/buffer/Buffer.ts),
[legacy ConPTY flags](https://github.com/microsoft/terminal/blob/v1.19.11213.0/src/winconpty/winconpty.h),
and [Unix window-size notification](https://man7.org/linux/man-pages/man2/TIOCSWINSZ.2const.html).

The native fixture remains alive during resize, uses bounded output/deadlines and
title-only readiness acknowledgments, and checks exact logical rows and prompt
adjacency. Parser fragmentation, selection/search, navigation, snapshot/pixel,
benchmark and native desktop evidence are distinct requirements. Native macOS,
Linux desktop and unexecuted host versions must not be inferred from a Windows
or WSL process test.
