# Command-result surface assurance

This page owns the focused correctness and native visual evidence for completed-
command output grouping. User behavior and shell support remain authoritative in
[Shell integration](SHELL-INTEGRATION.md#prompt-ownership); the roadmap status
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
   boundary; the renderer deduplicates source and boundary anchors by result ID;
6. expected output tokens must be present in renderer-neutral visible text;
7. known success/error state must match the completed command, while a shell
   that cannot expose it must remain explicitly neutral;
8. every accepted desktop completion carries one terminal-owned local datetime
   and Unix-millisecond identity captured at completion; source and following-
   prompt boundary metadata must agree, while render-width fallback may omit
   duration or shorten the visible date without changing that identity;
9. silent completion remains semantic without creating an empty surface or
   borrowing the preceding result;
10. a native glyph-only region, excluding divider/status decoration, must
   contain real output paint;
11. blank surface pixels must visibly differ from adjacent blank gutter pixels;
12. WGPU and the independent CPU fallback must pass the same release contract;
13. generated snapshots and native hooks remain feature-gated test evidence and
    add no product I/O, PTY bytes, terminal rows, persistence, or telemetry.

Test-first execution also exposed a separate PowerShell defect. A shell-only
failure could observe a stale successful `LASTEXITCODE` from an earlier native
process and publish success styling. The prompt now captures the completed
pipeline state and preserves an exact native process exit code such as `7`
without changing the user-owned `LASTEXITCODE`. A focused regression reproduces
the stale-zero case and the exact wrapped-native failure before invoking the
next prompt.

## Automated contract

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
```

Command-boundary navigation reuses this metadata without changing result
ownership. VT tests cover no marks, one/multiple marks, first/last no-ops,
wrapped prompt runs, adjacent prompts after silent commands, directional
navigation from output, reflow, and retained history. Platform binding tests
cover Windows, Linux/BSD, and macOS defaults
plus search/Vi/alternate-screen suppression. The native Windows driver sends
the real foreground Ctrl+Shift+Up/Down modifier sequence and independently
compares the selected route, per-pane display offsets, live cursor prompt
generation, raw
cursor line, and unfocused pane before and after both directions.

Native Windows validation:

```text
cargo build -p automexia-terminal --bin automexia --locked --features visual-test-hooks
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tests/integration/resize-stress-windows.ps1 -Binary target/debug/automexia.exe -ResourceReport <private-wgpu-report> -ResultCapture <private-wgpu-png>
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tests/integration/resize-stress-windows.ps1 -Binary target/debug/automexia.exe -ResourceReport <private-cpu-report> -ResultCapture <private-cpu-png> -UseCpuRenderer
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tests/integration/session-clone-wsl-windows.ps1 -Binary target/debug/automexia.exe -Distribution Ubuntu-24.04
```

The screenshots and path-bearing reports are private, ignored local evidence.
Portable QA bundles exclude terminal-content PNGs and retain only bounded,
redacted summaries.

## Recorded local evidence

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
resource/elevated suites, five exact 9,216-capture visual matrices, Narrator,
NVDA, VoiceOver, and Orca X11/Wayland sessions, and independent review must
populate one manifest that passes
`python tools/ci/s1_assurance.py validate --require-complete`.
Unintegrated or unsupported shells continue to fail closed.

## Relationship to planned diagnostic navigation

The proposed
[Semantic Diagnostic Navigator](SEMANTIC-DIAGNOSTIC-NAVIGATOR.md) may reuse
trusted prompt result metadata for exact failed-command traversal after v0.4.
The stable following-prompt boundary closes the visible overflow defect but must
not be described as a durable complete command-output region. Its DN1 slice
navigates to the identified prompt/input anchor only; generic
error-section reconstruction is separate DN2/DN3 work over bounded normal
scrollback.

Failed-command status and recognized Error/Fatal output remain separate
classifications. Unknown CMD or unsupported-shell status stays neutral, and no
future navigator may infer failure from color or fabricate missing metadata.

This local evidence does not justify a universal native-platform claim. Native
Linux/macOS GUI frames, the complete theme/high-contrast matrix, and
Narrator/NVDA, VoiceOver, and Orca delivery remain outstanding under U10. The
WSL lifecycle runs do not substitute for native Linux desktop or macOS evidence.
