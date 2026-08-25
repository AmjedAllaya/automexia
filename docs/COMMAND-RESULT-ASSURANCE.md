# Command-result surface assurance

This page owns the focused correctness and native visual evidence for completed-
command output grouping. User behavior and shell support remain authoritative in
[Shell integration](SHELL-INTEGRATION.md#prompt-ownership); the roadmap status
remains authoritative in [UI branding roadmap](UI-BRANDING-ROADMAP.md).

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


The remediation changes the acceptance contract rather than merely adjusting a
threshold:

1. every output-producing command must publish a result key newer than the
   previous result;
2. an identified result generation must own the immediately preceding prompt;
3. any terminal row reused by unrelated output must retire stale prompt and
   result metadata before that output can be selected;
4. expected output tokens must be present in renderer-neutral visible text;
5. known success/error state must match the completed command, while a shell
   that cannot expose it must remain explicitly neutral;
6. silent completion must exist semantically without creating an empty surface
   or borrowing the preceding result;
7. a native glyph-only region, excluding divider/status decoration, must
   contain real output paint;
8. blank surface pixels must visibly differ from adjacent blank gutter pixels;
9. WGPU and the independent CPU fallback must pass the same contract;
10. generated snapshots and native hooks remain feature-gated test evidence and
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
| interactive CMD listing | fresh neutral surface with no fabricated generation, status, or duration |
| native WSL Bash stdout/multiline/stderr/silent matrix | the same visible-or-silent ownership rules through Automexia, ConPTY, WSL, Bash, VT, and renderer |

Focused reproduction and ownership checks:

```text
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File tools/ci/test_shell_integration.ps1
cargo test -p automexia-terminal --bin automexia --locked --features native-gui-test-hooks result_
cargo test -p rio-vt --locked semantic_
wsl.exe --distribution Ubuntu-24.04 --exec bash tools/ci/test_shell_integration.sh
wsl.exe --distribution Ubuntu-24.04 --exec zsh tools/ci/test_zsh_integration.zsh
wsl.exe --distribution Ubuntu-24.04 --exec fish tools/ci/test_fish_integration.fish
cargo bench -p rio-vt --bench vt_input command_result_lifecycle -- --noplot
```

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

Fresh Windows runs on 2026-08-25 produced:

| Evidence | WGPU | CPU fallback |
|---|---:|---:|
| PowerShell command cases | 8 passed | 8 passed |
| interactive CMD neutral-output case | passed | passed |
| blank surface/gutter RGB distance | 27 | 27 |
| glyph color buckets | 38 | 38 |
| glyph luminance spread | 211 | 211 |
| handle growth | 73 | 73 |
| thread growth | 13 | 13 |
| private-byte growth | 47,636,480 | 47,296,512 |
| working-set growth | 28,549,120 | 28,995,584 |
| descendant-process growth | 10 | 10 |

Both native runs stayed below the existing ceilings of 384 handles, 48 threads,
536,870,912 private bytes, 536,870,912 working-set bytes, and 16 descendants.
The WGPU and CPU frames were visually inspected and showed ordinary, multiline,
external, parameter-binding error, provider/pipeline, native-stderr, and later
history results with persistent bands, gutters, rules, and success/failure
badges and no vertical rail. CPU glyph-area pixel evidence matched WGPU.

The real WSL clone gate also passed Bash stdout, multiline pipeline, stderr
exit `7`, and silent-success cases through the native Windows application. The
native Bash, Zsh, and Fish lifecycle harnesses passed in Ubuntu 24.04 WSL.
Focused parser measurements on the same host processed 256 verified result
lifecycles at 326.72-335.89 microseconds (762.15K-783.54K elements/second) and
256 boundary-only lifecycles at 216.05-226.51 microseconds
(1.1302M-1.1849M elements/second). Criterion detected no significant change
against the immediately preceding same-host samples (`p=0.97` and `p=0.88`). A
controlled release regression claim still requires the repository baseline.

## Remaining gates

The command-result source contract is **Partially done** in the UI roadmap.
Existing Fish prompt/preexec/postexec, CMD neutral-close, PowerShell, and short
WSL fixtures pass their bounded cases. A real gap remains when output exceeds
the visible viewport: the originating prompt row can enter scrollback while the
renderer scans only visible rows, which can leave commands such as GNU/WSL
`ls -ll` without a result surface. Completion requires a failing real
parser-to-scrollback-to-visible-render regression at viewport-minus-one,
viewport, viewport-plus-one, large, and storm heights, followed by the ownership
fix and rerun of native shell, exact-pixel, accessibility, resource, and
benchmark evidence. Unintegrated or unsupported shells still fail closed.

This local evidence does not justify a universal native-platform claim. Native
Linux/macOS GUI frames, the complete theme/high-contrast matrix, and
Narrator/NVDA, VoiceOver, and Orca delivery remain outstanding under U10. The
WSL lifecycle runs do not substitute for native Linux desktop or macOS evidence.
