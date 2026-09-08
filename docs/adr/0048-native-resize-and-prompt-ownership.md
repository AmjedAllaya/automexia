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

## Alternatives and evidence

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
