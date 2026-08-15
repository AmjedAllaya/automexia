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

## Author audit: 2026-08-13 through 2026-08-15

On 2026-08-15, maintainers fetched `rio-upstream/main` and audited every commit
authored by Raphael Amorim from the start of 2026-08-13 through the audit time.
The compared upstream head was `926a398e2bcf3641c49cdfa13a11a837a574eea4`;
the calendar window contained 19 commits. Commit IDs below are evidence, not a
claim that Automexia can safely merge the upstream branch.

| Upstream commits | Disposition in Automexia | Evidence or prerequisite |
|---|---|---|
| `926a398` | Already superseded. Exit events are resolved by exact route across pane-local tabs, splits, and background window tabs. Removal updates the selected route and index, while delayed intentional-close events are acknowledged only once for their exact route. | Context-manager route isolation and intentional-close regressions. Automexia deliberately retains the neighboring-tab selection rule of its own window/pane model. |
| `8cd6d36` | Adapted. The desktop binary weak-links CoreGraphics on macOS targets without lowering Automexia's declared macOS 11 deployment floor or affecting Windows resources. | Native macOS compile/package jobs plus a repository policy test that rejects removal or hard-link drift. |
| `8784acc`, `4242d04` | Adapted by API contract. Private `librio` now forwards mouse press/release and 1002/1003 motion, preserves SGR and legacy release encoding, supports Shift-to-select bypass, rejects invalid buttons, and exposes null-safe C entry points and header declarations. | Cross-platform encoding/C-boundary tests, Unix PTY ownership tests, and the compiled C smoke source. |
| `5432ea8`, `a0fb6da` | Deferred as an optional visual redesign. Automexia's trail owns pane route changes and explicit geometry-change snapping, neither of which exists in the upstream replacement. Replacing 900+ renderer lines without preserving those contracts would reintroduce cross-pane trails during resize. | Requires an ADR, deterministic thin/block/beam geometry goldens, route-switch and resize-storm coverage, frame-time/allocation benchmarks, and controlled GPU visual evidence. The current bounded trail remains supported. |
| `7f1178b`, `916ef7f`, `70eb1bb`, `547a485`, `a7562aa` | Already adapted in the preceding ledger. | Exact visible-match latch, terminal damage, repaint, modifier refresh, and terminal-mouse ownership tests. |
| `b489755` | Adapted where independent of the deferred grapheme model. A regression now proves extras-table reclamation removes stale interning entries before equal content is reallocated. | `rio-vt` reclamation regression. The accompanying prepend limitation belongs to Rio's absent grapheme-LUT path. |
| `62015b6`, `faff841`, `dca057d` | Deferred with the mode-2027 grapheme architecture. The public `cluster_width` API depends on Rio's `rio-unicode`, `grapheme_lut`, and `Mode::GRAPHEME_CLUSTER`; exposing it over Automexia's retained legacy grid would promise parity it cannot provide. | Dedicated ADR, Unicode/terminal width corpus, C/Wasm parity, fuzzing, renderer goldens, and migration of semantic prompt/selection/reflow contracts. |
| `821cebd` | Not applicable. It is Rio's `0.5.24` release-version change; Automexia owns its `0.4.x` product version and private crate policy. | Identity, provenance, and package metadata gates. |
| `be39b89`, `e2e3045` | Already adapted in the preceding ledger to the retained legacy width path. | VS15/VS16 emoji-base and width-stability regressions. |
| `84c64d9` | Already adapted and branded in the preceding ledger. | DCS/CSI synchronized-update interoperability, fragmentation, timeout, terminfo, and identity tests. |

This audit therefore selected three coherent changes: the macOS weak-link,
complete embedded mouse input, and the independent extras interning regression.
It did not import Rio version metadata or incomplete slices of renderer/Unicode
architectures. Future audits start after `926a398` and must reevaluate deferred
series against Automexia's then-current ADRs and test evidence.
