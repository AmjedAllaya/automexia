# ADR 0014: Interactive, bounded local image quick look

- Status: Accepted for v0.4
- Date: 2026-08-14
- Owners: terminal core, frontend, renderer, and security maintainers

## Context

Automexia already inherits Rio's Sixel, Kitty Graphics, and iTerm2 inline image
support. Those protocols are application-driven: the PTY client deliberately
transmits pixels and placement metadata. They do not make a filename printed by
`ls` interactive.

Terminal users also expect direct filename previews in file managers and
terminal-native Quick Look workflows. Plain hover must still treat printed
paths as untrusted: discovery cannot perform filesystem work, network/UNC paths
must remain forbidden, and pointer movement must not allocate/decode
unpredictably.

## Decision

Keep terminal graphics protocols as the primary application integration and add
a separate Automexia-owned local quick-look overlay:

1. IO-free hit testing recognizes visible local raster-looking paths. A stable
   100 ms plain-hover target submits bounded background work; a direct click
   bypasses the delay and pins the card.
2. A pinned card consumes unmodified `Up`/`Left` and `Down`/`Right` to browse
   previous/next visible image paths, with wraparound. `Esc` dismisses it.
   Hover-only cards never capture arrow keys.
3. A terminal application in mouse-reporting mode keeps input ownership unless
   `Shift`, the established host-UI override, is held. A consumed preview press
   and release use one pane-local latch so the child never receives half an
   event pair.
4. `PreviewSelectedImage` is a stable binding action, mapped to Cmd+Alt+I on
   macOS and Ctrl+Alt+I elsewhere, and exposed in the command palette.
5. Only local regular raster files are accepted. Relative paths require trusted
   current/launch directory metadata; WSL translation requires the current
   validated distro. URLs, arbitrary UNC paths, SVG/PDF, symlinks, pipes, and
   device files are rejected.
6. File and decode bounds are fixed and dimensions are rejected before full
   pixel decode. Decode/downscale runs on one worker behind a 16-owner
   latest-request queue; the UI, PTY, and render threads never read or decode.
7. Every window owns a generation token and one bounded completion mailbox. A
   newer request replaces queued work for that owner; obsolete active completion
   is discarded without a shared result queue, cross-window delivery, or eviction.
8. Decoded thumbnails use an access-ordered 16-entry/32 MiB cache keyed by the
   opened path, length, and modification time. Pixel storage is shared with
   Sugarloaf and stable render identity permits normal GPU-texture reuse.
9. The overlay is renderer-owned and responsive. It does not modify terminal
   cells, selection, scrollback, prompt semantics, PTY size, or shell state.
10. Protocol decoders keep independent limits. iTerm2 now validates base64/file
   size and bounded decode dimensions/allocation before creating graphics.

No user-facing configuration is added in v0.4. Existing binding overrides can
replace the default action normally.

## Verification

Acceptance requires unit tests for candidate parsing, quoted/Unicode paths,
remote/control rejection, WSL mapping, file/dimension/allocation bounds, magic
format validation, changing-file invalidation, cache byte/count/LRU behavior,
latest-request fairness/capacity/cancellation, zero-copy renderer ownership,
native
small-image sizing, edge/tiny/large geometry, route/generation stale-result
rejection, action parsing, platform shortcut collisions, palette discovery,
and iTerm2 declared-size/dimension rejection. Architecture, identity,
warning-denied Clippy, workspace tests, resize stress, and native visual review
remain required gates.

## Consequences

Users gain both common workflows: rich TUI/CLI protocol images and a direct
preview of a printed or selected local filename. Warm validated previews avoid
decode, resize, pixel copy/hash, and redundant GPU upload. Pointer hit testing
stays IO-free; only a stable candidate submits bounded worker work. The first
release deliberately excludes remote
fetching, SVG/PDF rendering, directory galleries, editing, and animation UI;
those require separate threat models and product decisions.
