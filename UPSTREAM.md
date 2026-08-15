# Upstream policy

The `rio-upstream` remote tracks https://github.com/raphamorim/rio. Automexia's
audited base is `7d595af583f6ef1ea6036a66b367ba1e5a84d4a2`, tagged locally as
`rio-base-0.5.20-7d595af`.

Upstream updates begin on a short-lived compatibility branch. Maintainers review
the upstream range, dependency/security changes, platform behavior, and conflicts,
then selectively port coherent commits into Automexia with upstream commit IDs in
the commit or PR description. Run the complete conformance and platform matrix
before merging.

Never merge a moving Rio branch directly into stable Automexia. Do not rewrite
upstream authorship, remove inherited headers, or replace the upstream MIT notice.
The legacy bootstrap package remains an external downloadable archive and is not
part of the standalone development workflow.

## Adaptation ledger: 2026-08-10 through 2026-08-13

The first post-fork audit compared `rio-base-0.5.20-7d595af` with
`rio-upstream/main` at `7f1178b77c61828c0ed2c6ab68e9d0d5b6e947e9`. The
commits below were selected for correctness, latency, and interoperability.
They were adapted by behavior instead of cherry-picked because Automexia owns a
different frontend path, semantic prompt renderer, pane model, terminfo entry,
and test command surface.

| Upstream commits | Automexia adaptation | Regression evidence |
|---|---|---|
| `390a66f`, `531b897`, `e677252` | Kitty queries and placements now report real decoder/store results; delete actions remove virtual placements; atlas releases return bytes to the shared graphics budget only after both screens release the key. | Protocol response/quiet-level tests, real image-store placement tests, virtual-delete coverage, and two-screen memory-accounting tests in `rio-vt` and `rio-backend`. |
| `574f120` | Dim/bold intensity no longer mutates explicit cell backgrounds. Named/indexed colors retain the expected inverse-video intensity rules. | Renderer tests cover explicit, indexed, named, inverse, dim, and bold combinations. |
| `3b29117`, `b0db5e1` | Appending a combining mark to an already-painted cell damages that row immediately. | A VT damage regression asserts the visible row is repainted. |
| `c79d6ce`, `5155757` | OSC, APC, SOS, PM, and DCS payload runs use bulk scanning and slice delivery while preserving bytewise state-machine behavior at every control boundary. | Exhaustive `0x00..=0xff` scanner parity plus whole, bytewise, and deterministic fragmented-stream comparisons, including large payloads. |
| `a8516f7`, with the test intent of `7470c32` and `9a9ba9c` | Scrolled-back viewport offsets remain valid during sub-region scrolling, history saturation, resize, and incremental-damage consumption. | Viewport/history unit tests, a renderer-neutral convergence harness with 200 seeded parser/render races, and `cargo xtask test resize-stress` (2,000 grid transitions and 1,000 queued PTY resizes). |
| `84c64d9` | DCS synchronized-update begin/end forms share the existing CSI buffering lifecycle, tolerate fragmented input, and advertise `Sync` in Automexia's terminfo without importing upstream product metadata. | CSI/DCS interoperability, fragmentation, timeout, and flush tests; identity verification requires the exact terminfo capability. |
| `a7562aa`, `547a485`, `70eb1bb`, `916ef7f`, `7f1178b` | Modifier changes refresh link hints without pointer movement. A primary click latches one exact visible match, suppresses terminal mouse reporting for both event halves, opens only on a matching plain release, damages cleared highlights, and requests the repaint. Chrome and panel routing cannot retain a stale latch. | Exact hint-identity and mouse-ownership tests plus all-target desktop tests under mouse-mode-compatible routing. |
| `e2e3045`, `be39b89` | In the retained legacy wcwidth path, VS15/VS16 attach only to emoji bases and never change logical cell width. This prevents tmux and shell redraw math from diverging without importing the deferred mode-2027 architecture. | Narrow and wide emoji, non-emoji filtering, combining marks, pending-wrap, sequential-selector, and following-character VT regressions. |

The audit intentionally deferred the mode-2027/grapheme-cluster architecture
and the new public `cluster_width` API. Only the two independent legacy-selector
corrections were adapted. The remaining series replaces core cell/extras and
width contracts that Automexia's semantic prompt reflow, selection, renderer,
and extension boundaries currently depend on. It requires a dedicated ADR,
cross-platform width corpus, fuzzing, renderer goldens, and migration work; it
must not be smuggled into a correctness port. Upstream release preparation,
Nix/MSYS cache policy, website/Canario work, and Rio frontend metadata are not
applicable to this repository. The hidden-tab titlebar fix is also not copied:
Automexia's custom chrome has a separate tested double-click and resize model.

Every later audit must append a dated section with the compared head, selected
hashes, local behavioral changes, test evidence, and explicit deferrals. This
keeps selective ports reviewable without pretending the histories are directly
mergeable.
