# Command-result surface assurance

This page owns the focused correctness and native visual evidence for completed-
command output grouping. User behavior and shell support remain authoritative in
[Shell integration](SHELL-INTEGRATION.md#shell-ownership); the roadmap status
remains authoritative in [UI branding roadmap](UI-BRANDING-ROADMAP.md).

Generic result geometry, exit classification, color, spacing, the 540 ms
completion pulse, terminal-owned completion datetime, route-isolated state,
and native hooks are owned by
`apps/automexia-terminal/src/renderer/command_results.rs`. They are core
application-renderer feedback and remain active when the optional DevOps context
extension is disabled. `renderer/devops_status.rs` owns only optional prompt
context and semantic status. ADR 0035 and the feature-ownership mutation suite
enforce this separation.

## Validation incident

The first native assertion proved that result geometry existed and that the
combined result rectangle contained varied pixels. That was insufficient: text
or the divider could satisfy color diversity even if the resting surface itself
was imperceptible. It also did not prove that the surface belonged to the
command just completed. A later command matrix still missed another real
failure: full-screen or formatter output could overwrite an old prompt row
without retiring its semantic result identity, allowing the renderer to select
the stale command. Completion was reported from incomplete evidence, and manual
use exposed both gaps.
The full workspace suite then exposed a third gap: an identified but blank
prompt marker was being treated as proof of a single-row command. Output bounds
now require either an owned prompt-continuation row or visible content on the
identified prompt row. Empty markers fail closed instead of claiming the next
unowned row.

A fourth gap remained at viewport and retention boundaries. The renderer looked
only for result metadata on visible source prompts. Long output could move that
prompt into scrollback or evict it completely, leaving the following prompt and
command output visible without grouping. The terminal now publishes a stable,
content-free completion boundary on the following prompt; renderer evidence
therefore follows terminal lifecycle identity instead of guessing from visible
row position.

A fifth gap appeared when command navigation followed a resize. One prompt row
can legitimately carry the preceding command's boundary and its own completed
command metadata. The old native hook exposed only the selected latest result,
so two labels could be drawn at the same coordinates without failing the test.
The renderer now normalizes the visible projection: every result ID and display
row has one paint owner, the truthful preceding-output boundary wins over a
source-row fallback, and malformed ordering cannot change the decision. The
native snapshot exposes every issued badge rectangle so overlap is checked
instead of inferred from one selected surface.

A sixth gap was visible only in the captured native frame. One result badge was
unique, but it still overlapped optional prompt-context chips on the same row.
The two renderers had independent width assumptions: prompt context reserved 112
logical pixels while a full result label could exceed 250. Both now obey one
renderer-neutral 288-pixel right reservation with a guaranteed 10-pixel gap and
responsive result-label fallback. Native hooks expose every prompt chip and
result label rectangle, and the driver rejects cross-owner intersection.

A seventh gap was in the evidence path itself. The native JSON checkpoint could
be written before its matching CPU or GPU frame was presented, so a capture
could pair correct geometry with partially rasterized pixels. An escaped native
error dialog could also make two backend captures identically wrong. Test
checkpoints are now staged with a 4 MiB ceiling, published only after the
matching frame presents, and discarded when no frame or drawable is available.
Retained captures require Automexia foreground ownership, reject an overlapping
native dialog, and require two consecutive identical full-frame SHA-256 pixel
digests before the artifact is accepted.

An eighth gap appeared only after the evidence path was made strict. The CPU
renderer's frame-skip identity omitted the physical framebuffer extent. A
content-preserving Windows client resize could therefore enlarge the native
surface while the renderer skipped the repaint, leaving the new bottom rows
black until later input changed another hashed value. CPU frame identity now
includes width and height, and a frame becomes reusable only after
`softbuffer` presents it successfully. Unit coverage proves extent changes
cannot skip and unrecorded frames remain retryable; native WGPU/CPU comparison
proves the complete initial client and the post-navigation client match.

The remediation changes the acceptance contract rather than merely adjusting a
threshold:

1. every valid completion receives a stable pane-local result ID newer than the
   previous completion;
2. an identified result generation owns its source prompt while that row is
   retained;
3. output-producing completions also publish one boundary on the following
   prompt, carrying the source generation and stable result ID;
4. source and boundary metadata survive row reuse, active-prompt repaint and
   reflow, while stale rows and obsolete generations are retired;
5. viewport or complete source-prompt eviction must not remove the one visible
   boundary; the renderer deduplicates source and boundary anchors by result ID
   and emits at most one non-overlapping badge owner per display row;
6. optional prompt-context chips stop before the shared result reservation; a
   result label and every context-chip rectangle remain disjoint at every width;
7. expected output tokens must be present in renderer-neutral visible text;
8. known success/error state must match the completed command, while a shell
   that cannot expose it must remain explicitly neutral;
9. every accepted desktop completion carries one terminal-owned local datetime
   and Unix-millisecond identity captured at completion; source and following-
   prompt boundary metadata must agree, while render-width fallback may omit
   duration or shorten the visible date without changing that identity;
10. silent completion remains semantic without creating an empty surface or
   borrowing the preceding result;
11. a native glyph-only region, excluding divider/status decoration, must
   contain real output paint;
12. blank surface pixels must visibly differ from adjacent blank gutter pixels;
13. WGPU and the independent CPU fallback must pass the same release contract;
14. native readiness is published only after the matching frame presents, and
    retained frames require two identical full-pixel captures with no native
    dialog obscuring the client area;
15. generated snapshots and native hooks remain feature-gated test evidence and
    add no product I/O, PTY bytes, terminal rows, persistence, or telemetry.

Test-first execution also exposed a separate PowerShell defect. A shell-only
failure could observe a stale successful `LASTEXITCODE` from an earlier native
process and publish success styling. The prompt now captures the completed
pipeline state and preserves an exact native process exit code such as `7`
without changing the user-owned `LASTEXITCODE`. A focused regression reproduces
the stale-zero case and the exact wrapped-native failure before invoking the
next prompt.

## Automated contract

### Command markers versus pane dividers

The current marker is a short inset accent, capped at 48 logical pixels and one
quarter of pane width, with an inset capped at 12 pixels. Structural dividers and
their resize targets are unchanged. `command_results/rows.rs` owns this draw-only
geometry; the anchor adapter, timestamp and pulse remain in `command_results.rs`.
This uses shape and location as well as colour, consistent with
[W3C guidance on use of colour](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html).
It does not claim a complete WCAG or native-accessibility assessment.

The parser-path regression first failed on the old 836-pixel rule in an
840-pixel pane. Adjacent success/failure/silent commands now retain their own
short markers. Tests compare literal geometry, exact controlled pixel coverage
at 100/125/200/300/400 percent scale, invalid/extreme geometry, scrollback and
resize/navigation. Silent commands keep status feedback without an output fill.
The fictional parser-derived specimen uses the bundled font; its software
geometry raster is not an actual desktop or GPU capture. The native Windows
driver checks short, inset geometry and retains its original gutter, opacity,
identity, text and cleanup checks. Current native frames and screen-reader
review remain external; older full-width native captures do not validate this
new appearance.

```text
cargo test -p automexia-terminal --bin automexia --locked renderer::command_results
cargo bench -p automexia-renderer-benchmarks --bench text_fit --locked -- command_boundary_marker --noplot
```

The benchmark compares the actual allocation-free geometry with the prior
calculation at narrow, ordinary and 8K widths. It does not measure frame latency.
One local Windows x64 release-profile run (30 samples per case) measured the
new marker at 4.51–5.06 ns versus 1.66–1.72 ns for the old calculation across
40/720/8192-pixel widths. The roughly 3 ns increase buys finite/overflow and
inset validation; there are no added draw calls, allocations or background work.
These same-run helper measurements are not a native responsiveness guarantee.

Manual review on each claimed native renderer: execute a short output, a failing
command and a silent command, then split right/down. The short command accents
must not look like continuous pane dividers or respond to divider dragging.
Resize narrow/wide and navigate previous/next command; timestamps stay with their
result, no marker reaches another pane, and copying retains exact command output.
Repeat with light/dark/custom colours and reduced motion, and retain exact
reviewed frames separately from automated geometry evidence.

The native command matrix covers:

| Case | Required result |
|---|---|
| single-line `Write-Output` | fresh success surface and visible token |
| two-line PowerShell output | one fresh multiline success surface and both tokens |
| exact external `cmd.exe /D /C` invocation | fresh success surface and native output token |
| `Write-Error` | fresh failure surface, visible error token, and failure styling |
| exact PowerShell `ls -ll` parameter error | fresh failure surface and `ParameterBindingException` output |
| `Get-Item` provider pipeline | fresh success surface, pipeline token, and filename |
| native stderr with exit `7` | fresh failure surface, stderr token, and exact exit status |
| silent successful provider lookup | semantic success with no empty or falsely borrowed surface |
| interactive CMD listing | fresh neutral surface with a completion datetime and no fabricated generation, status, or duration |
| native WSL Bash stdout/multiline/stderr/silent matrix | the same visible-or-silent ownership rules through Automexia, ConPTY, WSL, Bash, VT, and renderer |
| output heights around viewport and two-viewport boundaries | one stable result ID; exact offscreen source ownership where required; one following-prompt boundary; visible output; no duplicate surface |
| resize followed by repeated previous/next command navigation | one deterministic badge per result ID and display row; no intersecting draw rectangles; unchanged pane, prompt, command line, and PTY input |
| long prompt context plus a historical result label | shared right reservation; no result/context rectangle intersection; responsive label and chip truncation |
| state checkpoint followed by retained capture | checkpoint follows a successful present; two consecutive full-frame digests agree; native dialog/foreground checks pass |
| same content across a physical surface resize | CPU frame identity changes, every newly exposed row is painted, and only a successful present becomes skippable |
| complete source-prompt scrollback eviction | the following prompt retains one boundary with the matching source generation and result ID |
| newline-only and silent completions | newline-only output publishes a boundary; silent completion publishes no empty surface |

Focused reproduction and ownership checks:

```text
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tools/ci/test_shell_integration.ps1
cargo test -p automexia-terminal --bin automexia --locked --features native-gui-test-hooks result_
cargo test -p automexia-terminal --bin automexia --locked command_result
cargo test -p rio-vt --locked command_timestamp
cargo test -p rio-vt --locked semantic_
python tools/ci/check_feature_ownership.py
python tools/ci/test_feature_ownership.py
wsl.exe --distribution Ubuntu-24.04 --exec bash tools/ci/test_shell_integration.sh
wsl.exe --distribution Ubuntu-24.04 --exec zsh tools/ci/test_zsh_integration.zsh
wsl.exe --distribution Ubuntu-24.04 --exec fish tools/ci/test_fish_integration.fish
cargo bench -p rio-vt --bench vt_input command_result_lifecycle --locked -- --noplot
cargo bench -p rio-vt --bench vt_input command_result_viewport_overflow --locked -- --noplot
cargo bench -p rio-vt --bench vt_input command_prompt_jump_15000_rows --locked -- --noplot
cargo bench -p rio-vt --bench vt_input command_result_resize_navigation_256 --locked -- --noplot
```

Command-boundary navigation reuses this metadata without changing result
ownership. VT tests cover no marks, one/multiple marks, first/last no-ops,
wrapped prompt runs, adjacent prompts after silent commands, directional
navigation from output, reflow, and retained history. Platform binding tests
cover Windows, Linux/BSD, and macOS defaults
plus search/Vi/alternate-screen suppression. The native Windows driver resizes
immediately before sending the real foreground Ctrl+Shift+Up/Down modifier
sequence. It independently compares the selected route, per-pane display
offsets, live cursor prompt generation, raw cursor line, and unfocused pane
before and after both directions. Every freshly reflowed, previous, next, and
restored frame must also expose unique result IDs and pairwise non-intersecting
badge rectangles. The same oracle validates every prompt-context rectangle and
rejects cross-owner intersection. The controlled visual fixture freezes both
command datetime and duration before exact raster comparison. Snapshot
readiness follows a successful present, and each retained capture must stabilize
at the exact same full-frame pixel digest twice. It positions the complete
physical client on the capture display, rejects non-opaque pixels, and holds a
fixed live-editor sentinel without submitting it so both backends render the
same PTY-owned state. The driver records exact temporary-fixture process
identities before closing the owner, then requires
the application and every captured descendant to exit inside the six-second
multi-session ceiling.

Native Windows validation:

```text
cargo build -p automexia-terminal --bin automexia --locked --features visual-test-hooks
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tests/integration/resize-stress-windows.ps1 -Binary target/debug/automexia.exe -ResourceReport <private-wgpu-report> -ResultCapture <private-wgpu-png> -ResultNavigationCapture <private-wgpu-navigation-png>
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tests/integration/resize-stress-windows.ps1 -Binary target/debug/automexia.exe -ResourceReport <private-cpu-report> -ResultCapture <private-cpu-png> -ResultNavigationCapture <private-cpu-navigation-png> -UseCpuRenderer
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tests/integration/session-clone-wsl-windows.ps1 -Binary target/debug/automexia.exe -Distribution Ubuntu-24.04
```

The screenshots and path-bearing reports are private, ignored local evidence.
Portable QA bundles exclude terminal-content PNGs and retain only bounded,
redacted summaries.

## Recorded local evidence

On 2026-09-02, the final 256-command interaction benchmark measured alternating
23/120-column reflow, previous/next prompt navigation, and visible-snapshot
publication at 13.569-14.641 microseconds on this Windows x86_64 host after a
5-second warm-up and 10-second measurement (100 samples; two high-mild
outliers). A shorter immediately preceding run measured 15.092-16.675
microseconds; Criterion classified the controlled repeat as a 3.936-11.255%
improvement. Earlier same-day runs ranged from 12.070-12.545 to
14.857-15.557 microseconds. The timed VT production path did not change between
these later runs, so the complete range is retained as host-scheduling variance
rather than silently selecting the fastest result. This remains same-host
development evidence, not a cross-platform threshold or universal regression
claim.

On 2026-09-02, final native Windows x86_64 WGPU and CPU runs both passed the
complete interaction/resource driver after resizing to 85x21, navigating to a
historical command, returning to the live prompt, and restoring size. Each
checked frame issued one result rectangle, all result/context rectangles were
disjoint, and each retained artifact stabilized across two exact full-frame
digests. The 1600x950 result frames matched under the zero-tolerance policy:
0 of 1,520,000 pixels changed and maximum channel delta 0. The 1125x800
navigation frames also matched exactly: 0 of 900,000 pixels changed and maximum
channel delta 0. The driver independently proved the intended output tokens,
one metadata owner per row, glyph paint, contrast, an unobscured opaque client,
and unchanged live-editor state. Manual inspection of the private final frames
and independent release review remain outstanding; the frames and path-bearing
reports remain ignored local evidence.

The first post-fix cross-backend comparison failed because one capture contained
48,000 changed pixels in a 99,200-pixel region: correct model geometry had raced
an incomplete presented surface. That failure was preserved and led to the
post-present checkpoint contract. A later strict comparison found 929 changing
live-editor pixels and led to the fixed unsent input sentinel. The next exact
comparison found all 99,200 pixels in a 1,600-by-62 bottom band differed because
the CPU extent was absent from its frame identity; that failure produced the
extent-aware, successful-present-only cache contract. Native teardown also first measured about
10.1 seconds with four panes because contexts consumed their graceful budgets
sequentially. The final broadcast-first Windows runs completed owner and exact
descendant shutdown in 2.101-2.132 seconds, below the declared 6,000 ms ceiling,
with no tracked survivor. These are same-host observations, not universal
platform thresholds.

On 2026-08-26, the Windows x86_64 optimized benchmark above measured one
previous-and-next navigation pair across 15,000 retained output rows at
53.608–56.748 microseconds (100 samples; one high-mild outlier). This is
same-host development evidence, not a cross-platform release threshold.

The clean 2026-08-25 Windows WGPU and CPU runs remain evidence for the original
eight bounded PowerShell cases, CMD neutral output, exact glyph paint, and
resource ceilings. They predate the current resting-tint and overflow fix and
are not presented as current-commit U10 visual evidence.

Fresh source and focused evidence on 2026-08-26 produced:

- 508 `rio-vt` unit tests and 3 VT conformance tests passed;
- 13 direct command-result tests and the 15-case broad `result_` renderer
  filter passed, including a real
  parser-to-scrollback-to-visible-render path;
- output heights `rows-2`, `rows-1`, `rows`, `rows+1`, `2*rows-1`, `2*rows`,
  and `2*rows+1` retain one surface; cases at or beyond one viewport require
  the source owner offscreen and the visible boundary to match its result ID;
- full source-prompt retention eviction, newline-only output, silent completion,
  prompt repaint, row reuse, reflow, and source/boundary deduplication passed;
- the 512-output-row Criterion case measured 192.69-202.72 microseconds and
  50.384-53.005 MiB/s on this Windows x86_64 host, with 20 samples and six
  outliers. This first sample has no same-commit controlled baseline and is not
  a regression claim; and
- the current WGPU and CPU native drivers each passed the eight base PowerShell
  result cases, all seven dynamic viewport-boundary cases, CMD, resize, history,
  fullscreen, and multi-window stress. Each report recorded 15 completion
  timestamps; all 14 painted results exposed the exact frozen datetime label,
  while the silent result retained timestamp identity without an empty surface;
  and
- the real lifecycle benchmark measured verified-status completion at
  599.69-628.59 microseconds and boundary-only completion at 424.88-456.19
  microseconds over 20 samples on this Windows x86_64 host. This is a first
  timestamp-aware sample, not a controlled regression comparison.

The WGPU and CPU result captures and path-bearing reports remain ignored,
private local evidence. Automated geometry, label-state, glyph, contrast, and
resource assertions passed; independent human review of exact native frames is
still required before a controlled U10 visual claim.

## Remaining gates

The command-result source contract is **Fully done** in the UI roadmap. Stable
pane-local result identity and following-prompt boundaries remove the known
viewport and complete-source-eviction defect without scanning retained history,
adding product I/O, or fabricating output for silent commands.

U10 remains **Partially done** because release assurance is larger than this
source fix and one local automated Windows WGPU/CPU run. Native Linux X11 and
Wayland, native macOS Intel and Apple Silicon, other Windows GPU/RDP and named
resource/elevated suites, five exact 8,352-capture visual matrices, Narrator,
NVDA, VoiceOver, and Orca X11/Wayland sessions, and independent review must
populate one manifest that passes
`python tools/ci/s1_assurance.py validate --require-complete`.
Unintegrated or unsupported shells continue to fail closed.

This local evidence does not justify a universal native-platform claim. Native
Linux/macOS GUI frames, the complete theme/high-contrast matrix, and
Narrator/NVDA, VoiceOver, and Orca delivery remain outstanding under U10. The
WSL lifecycle runs do not substitute for native Linux desktop or macOS evidence.
