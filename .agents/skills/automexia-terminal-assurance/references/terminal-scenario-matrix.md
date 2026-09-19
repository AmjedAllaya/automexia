# Terminal scenario matrix

Select applicable scenarios by changed invariant. Do not treat this list as a requirement to run unrelated cases.

## Parsing, text, and protocol

- Empty, single-byte, fragmented, maximum, over-limit, malformed, truncated, repeated, interleaved, and cancelled sequences.
- UTF-8 split across reads, invalid UTF-8, combining marks, grapheme clusters, wide and zero-width characters, emoji sequences, bidi/control characters, and contextual case mapping.
- OSC, CSI, DCS, Kitty graphics, hyperlinks, clipboard, title, prompt, shell metadata, and other supported protocol boundaries.
- Parser recovery after malformed input and bounded buffering when terminators never arrive.
- Exact terminal cell semantics distinct from bytes, scalar values, graphemes, and pixels.

## PTY, shell, and process lifecycle

- Real supported shell launched with an exact executable and argument array.
- Startup failure before and after native handles or child identity exist.
- Child exit, pane close, window close, shutdown, cancellation, timeout, restart, and process-tree cleanup.
- Exact child identity, exit status, handle/thread recovery, and repeated capacity restoration.
- Windows ConPTY and Unix PTY differences, shell startup files, WSL boundaries, permissions, and environment inheritance.
- No command-string evaluation, unintended shell fallback, implicit Enter, or reassignment of another session to an abandoned PTY.

## Resize and reflow

- Keep the real shell and native PTY alive while changing both rows and columns.
- Verify a supposedly silent acknowledgment leaves exact rows and cursor position unchanged before resize.
- Wait for bounded native acknowledgment after each resize without injecting editor input or cleanup newlines.
- Include one- and two-cell viewports, mixed-axis sequences, repeated shrink/restore, large restoration, fixed-seed sequences, and resize storms.
- Include long padded rows exceeding viewport height, soft wraps, hard-line fill, explicit spaces, erased trailing content, history seams, selection, scrollback, prompts, and output arriving during resize.
- Assert exact row contents, order, uniqueness, cursor, prompt adjacency, successful child exit, and bounded history.
- Distinguish captured-stream reflow from native redraw evidence.

## Input and interaction

- Rapid keys, held repeat, modifiers, dead keys, IME composition, paste, clipboard, mouse, drag/drop, focus loss/restore, modal overlays, and route changes.
- Input isolation across panes, tabs, windows, sessions, and generations.
- No command activation or PTY bytes while an editor or modal owner consumes input.
- Stale queued actions rejected after route, focus, session, or generation replacement.

## Rendering and visibility

- Tiny through 8K viewports and 100–400% scale.
- CPU and WGPU paths where supported, theme variants, font reload, atlas/cache invalidation, and last-known-good fallback.
- Exact draw-state or pixel evidence only for controlled owned geometry and fonts.
- Long labels, truncation markers, Unicode shaping, selection, hyperlinks, images, overlays, and modal stacking.
- Semantic and accessible values retain full content even when presentation is fitted or clipped.

## Concurrency and resource behavior

- Zero, one, full, and over-capacity queues; cancellation before start and during work; stale generations; worker failure/restart; shutdown under load.
- Bounded task count and cleanup capacity as contracts separate from queue length.
- Repeated open/close and enable/disable cycles with stable memory, handles, threads, processes, caches, logs, temporary files, and persisted records.
- Timeouts retain cleanup ownership and do not publish stale results.
- Renderer wake follows state publication.

## Persistence and recovery

- Missing, stale, corrupt, partial, old-version, read-only, permission-denied, symlinked, replaced, disk-full, interrupted, and concurrent state.
- Atomic writes, migration, rollback, disable, uninstall, recovery, and restart.
- Credentials remain in platform or external stores and persisted values are opaque references.
- Real machine paths, usernames, hosts, environment values, and provider output never enter fixtures, snapshots, logs, or committed reports.

## Security and authority

- Path traversal, TOCTOU, command injection, control-character spoofing, secret disclosure, privilege escalation, hostile remote output, and silent fallback.
- Capability grant, revocation, expiry, wrong-session, wrong-generation, disable, uninstall, and cleanup failure.
- Extension or optional-feature degradation never blocks core terminal input/output.
- Logs are bounded, redacted, content-minimal, and actionable.

## Benchmarks and stress

- Correctness assertions execute inside benchmark iterations.
- Compare the same host, compiler, profile, features, fixture, and controlled environment.
- Separate parser/grid/snapshot timing from native PTY, input-to-frame, compositor, network, provider, and package latency.
- Reject corrupt output, unbounded history, resource growth, or incomplete cleanup even when elapsed time improves.
